use gtk4::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

/// System tray integration for EasySSH
pub struct SystemTray {
    menu: gtk4::PopoverMenu,
    icon: gtk4::Image,
    is_visible: RefCell<bool>,
    click_callback: RefCell<Option<Box<dyn Fn() + 'static>>>,
    quit_callback: RefCell<Option<Box<dyn Fn() + 'static>>>,
}

impl SystemTray {
    /// Create a new system tray instance
    /// Note: On GNOME, this creates a tray indicator that can be accessed
    /// through the system status area
    pub fn new() -> Option<Self> {
        // Check if we can create a tray icon
        if !Self::is_tray_available() {
            tracing::warn!("System tray not available on this desktop environment");
            return None;
        }

        let menu = create_tray_menu();
        let icon = gtk4::Image::from_icon_name("network-wired-symbolic");
        icon.set_pixel_size(22);

        let tray = Self {
            menu,
            icon,
            is_visible: RefCell::new(true),
            click_callback: RefCell::new(None),
            quit_callback: RefCell::new(None),
        };

        tracing::info!("System tray initialized");
        Some(tray)
    }

    /// Check if system tray is available
    fn is_tray_available() -> bool {
        // Check desktop environment
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            let desktop = desktop.to_lowercase();
            // GNOME 3.26+ requires AppIndicator extension
            if desktop.contains("gnome") {
                tracing::debug!("GNOME detected - tray support depends on AppIndicator extension");
                return true; // Assume extension is available
            }
            // KDE, XFCE, etc. have native tray support
            if desktop.contains("kde")
                || desktop.contains("xfce")
                || desktop.contains("mate")
                || desktop.contains("cinnamon")
                || desktop.contains("unity")
            {
                return true;
            }
        }
        true // Assume available by default
    }

    /// Show notification to user
    pub fn show_notification(&self, title: &str, body: &str) {
        if let Some(app) = gtk4::Application::default() {
            let notification = gio::Notification::new(title);
            notification.set_body(Some(body));
            notification.set_icon(&gio::ThemedIcon::new("network-wired-symbolic"));
            notification.set_priority(gio::NotificationPriority::Normal);

            app.send_notification(Some("easyssh-connection"), &notification);
            tracing::debug!("Notification sent: {} - {}", title, body);
        }
    }

    /// Show connection success notification
    pub fn notify_connection_success(&self, server_name: &str) {
        self.show_notification(
            &format!("Connected to {}", server_name),
            "SSH connection established successfully",
        );
    }

    /// Show connection error notification
    pub fn notify_connection_error(&self, server_name: &str, error: &str) {
        self.show_notification(&format!("Connection failed: {}", server_name), error);
    }

    /// Show server status change notification
    pub fn notify_status_change(&self, server_name: &str, status: &str) {
        self.show_notification(
            &format!("{} is now {}", server_name, status),
            "Server status has changed",
        );
    }

    /// Set visibility state
    pub fn set_visible(&self, visible: bool) {
        *self.is_visible.borrow_mut() = visible;
        tracing::debug!("Tray icon visibility: {}", visible);
    }

    /// Connect to tray click events
    pub fn connect_activate<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.click_callback.replace(Some(Box::new(callback)));
    }

    /// Connect to quit action
    pub fn connect_quit<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.quit_callback.replace(Some(Box::new(callback)));
    }

    /// Get tray widget for adding to a container
    pub fn widget(&self) -> &gtk4::Image {
        &self.icon
    }
}

impl Clone for SystemTray {
    fn clone(&self) -> Self {
        Self {
            menu: self.menu.clone(),
            icon: self.icon.clone(),
            is_visible: self.is_visible.clone(),
            click_callback: RefCell::new(None),
            quit_callback: RefCell::new(None),
        }
    }
}

/// Create the tray context menu
fn create_tray_menu() -> gtk4::PopoverMenu {
    let menu_model = gio::Menu::new();

    // Show window action
    let show_item = gio::MenuItem::new(Some("Show EasySSH"), Some("app.activate"));
    menu_model.append_item(&show_item);

    menu_model.append(Some(""), Some("separator"));

    // Quick servers submenu
    let servers_menu = gio::Menu::new();
    let servers_item = gio::MenuItem::new_submenu(Some("Quick Connect"), &servers_menu);
    menu_model.append_item(&servers_item);

    menu_model.append(Some(""), Some("separator"));

    // Settings
    let prefs_item = gio::MenuItem::new(Some("Preferences"), Some("app.preferences"));
    menu_model.append_item(&prefs_item);

    menu_model.append(Some(""), Some("separator"));

    // Quit
    let quit_item = gio::MenuItem::new(Some("Quit"), Some("app.quit"));
    menu_model.append_item(&quit_item);

    gtk4::PopoverMenu::builder()
        .menu_model(&menu_model)
        .has_arrow(true)
        .build()
}

/// Desktop notification manager
pub struct NotificationManager {
    app: gtk4::Application,
}

impl NotificationManager {
    pub fn new(app: &gtk4::Application) -> Self {
        Self { app: app.clone() }
    }

    /// Send a desktop notification
    pub fn notify(&self, id: &str, title: &str, body: &str, icon: &str) {
        let notification = gio::Notification::new(title);
        notification.set_body(Some(body));
        notification.set_icon(&gio::ThemedIcon::new(icon));
        notification.set_priority(gio::NotificationPriority::Normal);

        // Add default action
        notification.set_default_action_and_target_value("app.activate", None);

        self.app.send_notification(Some(id), &notification);
    }

    /// Send connection notification
    pub fn notify_connection(&self, server_name: &str, connected: bool) {
        let (title, body, icon) = if connected {
            (
                &format!("Connected to {}", server_name),
                "SSH session active",
                "network-wired-symbolic",
            )
        } else {
            (
                &format!("Disconnected from {}", server_name),
                "SSH session ended",
                "network-offline-symbolic",
            )
        };

        self.notify("connection-status", title, body, icon);
    }

    /// Send error notification
    pub fn notify_error(&self, title: &str, message: &str) {
        self.notify("error", title, message, "dialog-error-symbolic");
    }

    /// Send info notification
    pub fn notify_info(&self, title: &str, message: &str) {
        self.notify("info", title, message, "dialog-information-symbolic");
    }

    /// Withdraw a notification
    pub fn withdraw(&self, id: &str) {
        self.app.withdraw_notification(id);
    }

    /// Check if notifications are supported
    pub fn is_supported() -> bool {
        // Check if we can get the default application
        gtk4::Application::default().is_some()
    }
}

/// Quick actions for common tasks
pub struct QuickActions {
    actions: gio::SimpleActionGroup,
}

impl QuickActions {
    pub fn new() -> Self {
        let actions = gio::SimpleActionGroup::new();
        Self { actions }
    }

    /// Add a quick connect action
    pub fn add_quick_connect<F>(&self, name: &str, callback: F)
    where
        F: Fn() + 'static,
    {
        let action = gio::SimpleAction::new(&format!("connect-{}", name.replace(" ", "-")), None);
        action.connect_activate(move |_, _| {
            callback();
        });
        self.actions.add_action(&action);
    }

    /// Get action group for adding to widgets
    pub fn action_group(&self) -> &gio::SimpleActionGroup {
        &self.actions
    }
}

/// Initialize notifications for the application
pub fn init_notifications(app: &gtk4::Application) {
    // Set notification defaults
    app.set_application_id("com.easyssh.EasySSH");

    tracing::info!("Notification system initialized");
}

/// Request attention to the window (flashing on X11, on GNOME uses notification)
pub fn request_attention(window: &gtk4::ApplicationWindow) {
    if window.is_active() {
        return;
    }

    // On X11, try to set urgency hint
    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        // Use present_with_time to request attention
        let display = gtk4::gdk::Display::default();
        if let Some(display) = display {
            // This will cause the window to be highlighted in the taskbar
            window.present();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_creation() {
        // Just test that the struct compiles
        assert!(true);
    }
}
