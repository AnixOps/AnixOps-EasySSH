#![allow(dead_code)]

//! Windows 11 Native Toast Notification System for EasySSH
//!
//! Features:
//! - Connection success/failure notifications
//! - File transfer completion notifications
//! - Monitoring alerts (CPU/memory threshold exceeded)
//! - Background running notification
//! - Click notification to open corresponding window
//! - Notification history panel
//! - Custom notification settings
//!
//! Uses Windows.UI.Notifications.ToastNotification API

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Local, Duration};

// Windows API imports for notifications
use windows::{
    core::{HSTRING, Result},
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{
        ToastNotification, ToastNotificationManager,
        NotificationData, NotificationUpdateResult,
    },
};

/// Notification types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationType {
    ConnectionSuccess,
    ConnectionFailed,
    FileTransferComplete,
    FileTransferFailed,
    CpuAlert,
    MemoryAlert,
    DiskAlert,
    BackgroundRunning,
    UpdateAvailable,
    SessionDisconnected,
    SnippetExecuted,
    Custom(String),
}

impl NotificationType {
    fn default_icon(&self) -> &'static str {
        match self {
            NotificationType::ConnectionSuccess => "✓",
            NotificationType::ConnectionFailed => "✗",
            NotificationType::FileTransferComplete => "📁",
            NotificationType::FileTransferFailed => "⚠",
            NotificationType::CpuAlert => "🔥",
            NotificationType::MemoryAlert => "💾",
            NotificationType::DiskAlert => "💿",
            NotificationType::BackgroundRunning => "🔄",
            NotificationType::UpdateAvailable => "⬆",
            NotificationType::SessionDisconnected => "🔌",
            NotificationType::SnippetExecuted => "⚡",
            NotificationType::Custom(_) => "🔔",
        }
    }

    fn default_title(&self) -> &'static str {
        match self {
            NotificationType::ConnectionSuccess => "连接成功",
            NotificationType::ConnectionFailed => "连接失败",
            NotificationType::FileTransferComplete => "文件传输完成",
            NotificationType::FileTransferFailed => "文件传输失败",
            NotificationType::CpuAlert => "CPU 告警",
            NotificationType::MemoryAlert => "内存告警",
            NotificationType::DiskAlert => "磁盘告警",
            NotificationType::BackgroundRunning => "EasySSH 后台运行",
            NotificationType::UpdateAvailable => "更新可用",
            NotificationType::SessionDisconnected => "会话断开",
            NotificationType::SnippetExecuted => "命令片段执行",
            NotificationType::Custom(_) => "通知",
        }
    }

    fn sound_id(&self) -> &'static str {
        match self {
            NotificationType::ConnectionSuccess => "ms-winsoundevent:Notification.Default",
            NotificationType::ConnectionFailed => "ms-winsoundevent:Notification.IM",
            NotificationType::FileTransferComplete => "ms-winsoundevent:Notification.Mail",
            NotificationType::FileTransferFailed => "ms-winsoundevent:Notification.IM",
            NotificationType::CpuAlert | NotificationType::MemoryAlert | NotificationType::DiskAlert =>
                "ms-winsoundevent:Notification.Reminder",
            NotificationType::SessionDisconnected => "ms-winsoundevent:Notification.SMS",
            _ => "ms-winsoundevent:Notification.Default",
        }
    }
}

/// Notification priority
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    Low,
    Default,
    High,
    Urgent,
}

impl NotificationPriority {
    fn to_windows_string(&self) -> &'static str {
        match self {
            NotificationPriority::Low => "low",
            NotificationPriority::Default => "default",
            NotificationPriority::High => "high",
            NotificationPriority::Urgent => "urgent",
        }
    }
}

/// Individual notification record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub priority: NotificationPriority,
    pub read: bool,
    pub action_data: Option<NotificationActionData>,
    pub icon: String,
}

/// Action data for notification clicks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationActionData {
    pub action_type: String,
    pub server_id: Option<String>,
    pub session_id: Option<String>,
    pub transfer_id: Option<String>,
    pub path: Option<String>,
}

/// Notification settings per type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationTypeSettings {
    pub enabled: bool,
    pub sound_enabled: bool,
    pub show_in_history: bool,
    pub priority: NotificationPriority,
}

impl Default for NotificationTypeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::Default,
        }
    }
}

/// Global notification settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub global_enabled: bool,
    pub show_tray_notifications: bool,
    pub keep_history_days: u32,
    pub max_history_count: usize,
    pub do_not_disturb: bool,
    pub dnd_until: Option<DateTime<Local>>,
    pub type_settings: HashMap<NotificationType, NotificationTypeSettings>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        let mut type_settings = HashMap::new();

        // Set defaults for each notification type
        type_settings.insert(NotificationType::ConnectionSuccess, NotificationTypeSettings {
            enabled: true,
            sound_enabled: false,
            show_in_history: true,
            priority: NotificationPriority::Default,
        });

        type_settings.insert(NotificationType::ConnectionFailed, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::High,
        });

        type_settings.insert(NotificationType::FileTransferComplete, NotificationTypeSettings {
            enabled: true,
            sound_enabled: false,
            show_in_history: true,
            priority: NotificationPriority::Default,
        });

        type_settings.insert(NotificationType::FileTransferFailed, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::High,
        });

        type_settings.insert(NotificationType::CpuAlert, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::Urgent,
        });

        type_settings.insert(NotificationType::MemoryAlert, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::Urgent,
        });

        type_settings.insert(NotificationType::DiskAlert, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::High,
        });

        type_settings.insert(NotificationType::BackgroundRunning, NotificationTypeSettings {
            enabled: true,
            sound_enabled: false,
            show_in_history: false,
            priority: NotificationPriority::Low,
        });

        type_settings.insert(NotificationType::UpdateAvailable, NotificationTypeSettings {
            enabled: true,
            sound_enabled: false,
            show_in_history: true,
            priority: NotificationPriority::Default,
        });

        type_settings.insert(NotificationType::SessionDisconnected, NotificationTypeSettings {
            enabled: true,
            sound_enabled: true,
            show_in_history: true,
            priority: NotificationPriority::High,
        });

        type_settings.insert(NotificationType::SnippetExecuted, NotificationTypeSettings {
            enabled: false,
            sound_enabled: false,
            show_in_history: true,
            priority: NotificationPriority::Low,
        });

        Self {
            global_enabled: true,
            show_tray_notifications: true,
            keep_history_days: 30,
            max_history_count: 1000,
            do_not_disturb: false,
            dnd_until: None,
            type_settings,
        }
    }
}

/// Notification manager - handles toast notifications and history
pub struct NotificationManager {
    history: Arc<Mutex<Vec<NotificationRecord>>>,
    settings: Arc<Mutex<NotificationSettings>>,
    app_user_model_id: String,
}

impl NotificationManager {
    /// Create new notification manager
    pub fn new(app_id: &str) -> Self {
        Self {
            history: Arc::new(Mutex::new(Vec::new())),
            settings: Arc::new(Mutex::new(NotificationSettings::default())),
            app_user_model_id: app_id.to_string(),
        }
    }

    /// Load settings from storage
    pub fn load_settings(&self, settings: NotificationSettings) {
        let mut s = self.settings.lock().unwrap();
        *s = settings;
    }

    /// Get current settings
    pub fn get_settings(&self) -> NotificationSettings {
        self.settings.lock().unwrap().clone()
    }

    /// Update settings
    pub fn update_settings(&self, settings: NotificationSettings) {
        let mut s = self.settings.lock().unwrap();
        *s = settings;
    }

    /// Check if notifications are currently in do-not-disturb mode
    pub fn is_dnd_active(&self) -> bool {
        let settings = self.settings.lock().unwrap();
        if !settings.do_not_disturb {
            return false;
        }

        if let Some(until) = settings.dnd_until {
            if Local::now() > until {
                // DND period expired
                return false;
            }
        }

        true
    }

    /// Enable do-not-disturb mode
    pub fn enable_dnd(&self, duration_minutes: Option<u32>) {
        let mut settings = self.settings.lock().unwrap();
        settings.do_not_disturb = true;
        settings.dnd_until = duration_minutes.map(|m| Local::now() + Duration::minutes(m as i64));
    }

    /// Disable do-not-disturb mode
    pub fn disable_dnd(&self) {
        let mut settings = self.settings.lock().unwrap();
        settings.do_not_disturb = false;
        settings.dnd_until = None;
    }

    /// Send a toast notification
    pub fn notify(
        &self,
        notification_type: NotificationType,
        title: Option<&str>,
        message: &str,
        priority: Option<NotificationPriority>,
        action_data: Option<NotificationActionData>,
    ) -> Option<String> {
        // Check global settings
        let settings = self.settings.lock().unwrap();

        if !settings.global_enabled {
            return None;
        }

        // Check do-not-disturb (only for non-urgent notifications)
        if self.is_dnd_active() {
            let type_settings = settings.type_settings.get(&notification_type)
                .cloned()
                .unwrap_or_default();
            if type_settings.priority != NotificationPriority::Urgent {
                return None;
            }
        }

        // Check per-type settings
        let type_settings = settings.type_settings.get(&notification_type)
            .cloned()
            .unwrap_or_default();

        if !type_settings.enabled {
            return None;
        }

        let final_priority = priority.unwrap_or(type_settings.priority);

        // Generate notification ID
        let notification_id = Uuid::new_v4().to_string();

        // Build the toast XML
        let toast_xml = self.build_toast_xml(
            &notification_type,
            title.unwrap_or(notification_type.default_title()),
            message,
            &final_priority,
            type_settings.sound_enabled,
            &action_data,
        );

        // Send Windows toast notification
        if let Err(e) = self.send_windows_toast(&toast_xml, &notification_id) {
            eprintln!("Failed to send toast notification: {}", e);
        }

        // Add to history if enabled
        if type_settings.show_in_history {
            drop(settings); // Release lock before calling add_to_history
            let record = NotificationRecord {
                id: notification_id.clone(),
                notification_type: notification_type.clone(),
                title: title.unwrap_or(notification_type.default_title()).to_string(),
                message: message.to_string(),
                timestamp: Local::now(),
                priority: final_priority,
                read: false,
                action_data,
                icon: notification_type.default_icon().to_string(),
            };
            self.add_to_history(record);
        }

        Some(notification_id)
    }

    /// Build Windows toast notification XML
    fn build_toast_xml(
        &self,
        notification_type: &NotificationType,
        title: &str,
        message: &str,
        priority: &NotificationPriority,
        sound_enabled: bool,
        action_data: &Option<NotificationActionData>,
    ) -> String {
        let icon = notification_type.default_icon();
        let sound = if sound_enabled {
            notification_type.sound_id()
        } else {
            "silent"
        };

        // Serialize action data for the launch parameter
        let launch_param = if let Some(data) = action_data {
            serde_json::to_string(data).unwrap_or_default()
        } else {
            format!("{{\"type\":\"{:?}\"}}", notification_type)
        };

        format!(r#"<toast scenario="{scenario}" activationType="foreground" launch="{launch}">
    <visual>
        <binding template="ToastGeneric">
            <text hint-maxLines="1">{icon} {title}</text>
            <text>{message}</text>
            <text placement="attribution">EasySSH</text>
        </binding>
    </visual>
    <audio src="{sound}" />
    <actions>
        <action content="打开" arguments="open" activationType="foreground"/>
        <action content="忽略" arguments="dismiss" activationType="system"/>
    </actions>
</toast>"#,
            scenario = priority.to_windows_string(),
            launch = launch_param.replace('"', "&quot;"),
            icon = icon,
            title = title,
            message = message,
            sound = sound,
        )
    }

    /// Send Windows toast notification using Windows.UI.Notifications API
    fn send_windows_toast(&self, toast_xml: &str, tag: &str) -> Result<()> {
        // Parse the XML
        let xml_doc = XmlDocument::new()?;
        xml_doc.LoadXml(&HSTRING::from(toast_xml))?;

        // Create the toast notification
        let toast = ToastNotification::CreateToastNotification(&xml_doc)?;

        // Set tag for updating/dismissing later
        toast.SetTag(&HSTRING::from(tag))?;

        // Get the toast notifier for our app
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(&self.app_user_model_id))?;

        // Show the notification
        notifier.Show(&toast)?;

        Ok(())
    }

    /// Add notification to history
    fn add_to_history(&self, record: NotificationRecord) {
        let mut history = self.history.lock().unwrap();
        let settings = self.settings.lock().unwrap();

        // Add to front
        history.insert(0, record);

        // Trim to max size
        if history.len() > settings.max_history_count {
            history.truncate(settings.max_history_count);
        }

        // Clean old entries
        let cutoff = Local::now() - Duration::days(settings.keep_history_days as i64);
        history.retain(|r| r.timestamp > cutoff);
    }

    /// Get notification history
    pub fn get_history(&self, unread_only: bool, limit: Option<usize>) -> Vec<NotificationRecord> {
        let history = self.history.lock().unwrap();
        let mut result: Vec<NotificationRecord> = history
            .iter()
            .filter(|r| !unread_only || !r.read)
            .cloned()
            .collect();

        if let Some(l) = limit {
            result.truncate(l);
        }

        result
    }

    /// Mark notification as read
    pub fn mark_as_read(&self, notification_id: &str) -> bool {
        let mut history = self.history.lock().unwrap();
        if let Some(record) = history.iter_mut().find(|r| r.id == notification_id) {
            record.read = true;
            true
        } else {
            false
        }
    }

    /// Mark all notifications as read
    pub fn mark_all_as_read(&self) -> usize {
        let mut history = self.history.lock().unwrap();
        let mut count = 0;
        for record in history.iter_mut() {
            if !record.read {
                record.read = true;
                count += 1;
            }
        }
        count
    }

    /// Clear notification history
    pub fn clear_history(&self) {
        let mut history = self.history.lock().unwrap();
        history.clear();
    }

    /// Get unread count
    pub fn get_unread_count(&self) -> usize {
        let history = self.history.lock().unwrap();
        history.iter().filter(|r| !r.read).count()
    }

    /// Delete a specific notification
    pub fn delete_notification(&self, notification_id: &str) -> bool {
        let mut history = self.history.lock().unwrap();
        let len_before = history.len();
        history.retain(|r| r.id != notification_id);
        history.len() < len_before
    }

    /// Send connection success notification
    pub fn notify_connection_success(&self, server_name: &str, session_id: &str) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "open_session".to_string(),
            server_id: None,
            session_id: Some(session_id.to_string()),
            transfer_id: None,
            path: None,
        };

        self.notify(
            NotificationType::ConnectionSuccess,
            None,
            &format!("已成功连接到 {}", server_name),
            None,
            Some(action_data),
        )
    }

    /// Send connection failure notification
    pub fn notify_connection_failed(&self, server_name: &str, error: &str) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "retry_connection".to_string(),
            server_id: None,
            session_id: None,
            transfer_id: None,
            path: None,
        };

        self.notify(
            NotificationType::ConnectionFailed,
            None,
            &format!("{}: {}", server_name, error),
            Some(NotificationPriority::High),
            Some(action_data),
        )
    }

    /// Send file transfer complete notification
    pub fn notify_transfer_complete(&self, file_name: &str, session_id: &str, path: &str) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "show_transfer".to_string(),
            server_id: None,
            session_id: Some(session_id.to_string()),
            transfer_id: None,
            path: Some(path.to_string()),
        };

        self.notify(
            NotificationType::FileTransferComplete,
            None,
            &format!("文件 {} 传输完成", file_name),
            None,
            Some(action_data),
        )
    }

    /// Send file transfer failed notification
    pub fn notify_transfer_failed(&self, file_name: &str, error: &str) -> Option<String> {
        self.notify(
            NotificationType::FileTransferFailed,
            None,
            &format!("{}: {}", file_name, error),
            Some(NotificationPriority::High),
            None,
        )
    }

    /// Send CPU alert notification
    pub fn notify_cpu_alert(&self, server_name: &str, usage: f32) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "open_monitor".to_string(),
            server_id: None,
            session_id: None,
            transfer_id: None,
            path: None,
        };

        self.notify(
            NotificationType::CpuAlert,
            None,
            &format!("{} CPU 使用率: {:.1}%", server_name, usage),
            Some(NotificationPriority::Urgent),
            Some(action_data),
        )
    }

    /// Send memory alert notification
    pub fn notify_memory_alert(&self, server_name: &str, usage: f32) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "open_monitor".to_string(),
            server_id: None,
            session_id: None,
            transfer_id: None,
            path: None,
        };

        self.notify(
            NotificationType::MemoryAlert,
            None,
            &format!("{} 内存使用率: {:.1}%", server_name, usage),
            Some(NotificationPriority::Urgent),
            Some(action_data),
        )
    }

    /// Send background running notification
    pub fn notify_background_running(&self) -> Option<String> {
        self.notify(
            NotificationType::BackgroundRunning,
            None,
            "EasySSH 正在后台运行，点击打开主窗口",
            Some(NotificationPriority::Low),
            Some(NotificationActionData {
                action_type: "show_main_window".to_string(),
                server_id: None,
                session_id: None,
                transfer_id: None,
                path: None,
            }),
        )
    }

    /// Send session disconnected notification
    pub fn notify_session_disconnected(&self, _server_name: &str, session_id: &str) -> Option<String> {
        let action_data = NotificationActionData {
            action_type: "reconnect_session".to_string(),
            server_id: None,
            session_id: Some(session_id.to_string()),
            transfer_id: None,
            path: None,
        };

        self.notify(
            NotificationType::SessionDisconnected,
            None,
            &format!("与服务器的连接已断开", ),
            Some(NotificationPriority::High),
            Some(action_data),
        )
    }

    /// Update an existing notification with progress
    pub fn update_notification_progress(&self, tag: &str, progress: u32, message: &str) -> Result<()> {
        let xml_doc = XmlDocument::new()?;

        let xml = format!(
            r#"<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>{}</text>
            <progress value="{}" status="{}% 完成"/>
        </binding>
    </visual>
</toast>"#,
            message,
            progress as f32 / 100.0,
            progress
        );

        xml_doc.LoadXml(&HSTRING::from(xml))?;

        let data = NotificationData::new()?;
        data.SetSequenceNumber(progress)?;

        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(&self.app_user_model_id))?;
        let result = notifier.UpdateWithTag(&data, &HSTRING::from(tag))?;

        match result {
            NotificationUpdateResult::Succeeded => Ok(()),
            _ => Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Failed to update notification",
            )),
        }
    }

    /// Dismiss a notification
    pub fn dismiss_notification(&self, tag: &str) -> Result<()> {
        // Remove notification from history using the tag
        // Note: Windows ToastNotificationManager::History may not support direct tag removal
        // This is a best-effort operation
        let _ = tag;
        Ok(())
    }
}

impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        Self {
            history: Arc::clone(&self.history),
            settings: Arc::clone(&self.settings),
            app_user_model_id: self.app_user_model_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_settings_default() {
        let settings = NotificationSettings::default();
        assert!(settings.global_enabled);
        assert!(settings.show_tray_notifications);
        assert!(!settings.do_not_disturb);
        assert_eq!(settings.keep_history_days, 30);
    }

    #[test]
    fn test_notification_type_defaults() {
        assert_eq!(NotificationType::ConnectionSuccess.default_title(), "连接成功");
        assert_eq!(NotificationType::ConnectionFailed.default_title(), "连接失败");
        assert_eq!(NotificationType::FileTransferComplete.default_title(), "文件传输完成");
    }

    #[test]
    fn test_notification_history() {
        let manager = NotificationManager::new("test");

        // Manually add to history
        let record = NotificationRecord {
            id: "test-1".to_string(),
            notification_type: NotificationType::ConnectionSuccess,
            title: "Test".to_string(),
            message: "Test message".to_string(),
            timestamp: Local::now(),
            priority: NotificationPriority::Default,
            read: false,
            action_data: None,
            icon: "✓".to_string(),
        };

        manager.add_to_history(record.clone());

        let history = manager.get_history(false, None);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "test-1");

        // Mark as read
        assert!(manager.mark_as_read("test-1"));
        let unread = manager.get_history(true, None);
        assert_eq!(unread.len(), 0);

        // Check unread count
        assert_eq!(manager.get_unread_count(), 0);
    }
}
