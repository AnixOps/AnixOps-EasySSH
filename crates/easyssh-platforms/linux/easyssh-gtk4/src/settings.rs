use gtk4::prelude::*;
use std::cell::RefCell;

/// Application settings wrapper around GSettings
pub struct AppSettings {
    settings: gio::Settings,
    callbacks: RefCell<Vec<glib::SignalHandlerId>>,
}

impl AppSettings {
    /// Load settings from GSettings
    pub fn new() -> Option<Self> {
        let settings = gio::Settings::new("com.easyssh.EasySSH");

        Some(Self {
            settings,
            callbacks: RefCell::new(Vec::new()),
        })
    }

    // Window settings
    pub fn get_window_size(&self) -> (i32, i32) {
        let width = self.settings.int("window-width");
        let height = self.settings.int("window-height");
        (width, height)
    }

    pub fn set_window_size(&self, width: i32, height: i32) {
        let _ = self.settings.set_int("window-width", width);
        let _ = self.settings.set_int("window-height", height);
    }

    pub fn get_window_maximized(&self) -> bool {
        self.settings.boolean("window-maximized")
    }

    pub fn set_window_maximized(&self, maximized: bool) {
        let _ = self.settings.set_boolean("window-maximized", maximized);
    }

    // Sidebar settings
    pub fn get_sidebar_visible(&self) -> bool {
        self.settings.boolean("sidebar-visible")
    }

    pub fn set_sidebar_visible(&self, visible: bool) {
        let _ = self.settings.set_boolean("sidebar-visible", visible);
    }

    pub fn get_sidebar_width(&self) -> i32 {
        self.settings.int("sidebar-width")
    }

    pub fn set_sidebar_width(&self, width: i32) {
        let _ = self.settings.set_int("sidebar-width", width);
    }

    // Server list settings
    pub fn get_server_list_width(&self) -> i32 {
        self.settings.int("server-list-width")
    }

    pub fn set_server_list_width(&self, width: i32) {
        let _ = self.settings.set_int("server-list-width", width);
    }

    // Theme settings
    pub fn get_color_scheme(&self) -> String {
        self.settings.string("color-scheme").to_string()
    }

    pub fn set_color_scheme(&self, scheme: &str) {
        let _ = self.settings.set_string("color-scheme", scheme);
    }

    pub fn connect_color_scheme_changed<F>(&self, callback: F)
    where
        F: Fn(&str) + 'static,
    {
        let handler = self
            .settings
            .connect_changed(Some("color-scheme"), move |settings, _key| {
                let scheme = settings.string("color-scheme");
                callback(&scheme);
            });
        self.callbacks.borrow_mut().push(handler);
    }

    // Terminal settings
    pub fn get_terminal_emulator(&self) -> String {
        self.settings.string("terminal-emulator").to_string()
    }

    pub fn set_terminal_emulator(&self, emulator: &str) {
        let _ = self.settings.set_string("terminal-emulator", emulator);
    }

    // Connection settings
    pub fn get_connection_timeout(&self) -> i32 {
        self.settings.int("connection-timeout")
    }

    pub fn set_connection_timeout(&self, timeout: i32) {
        let _ = self.settings.set_int("connection-timeout", timeout);
    }

    // Notification settings
    pub fn get_show_notifications(&self) -> bool {
        self.settings.boolean("show-notifications")
    }

    pub fn set_show_notifications(&self, show: bool) {
        let _ = self.settings.set_boolean("show-notifications", show);
    }

    // Behavior settings
    pub fn get_confirm_deletions(&self) -> bool {
        self.settings.boolean("confirm-deletions")
    }

    pub fn set_confirm_deletions(&self, confirm: bool) {
        let _ = self.settings.set_boolean("confirm-deletions", confirm);
    }

    // State persistence
    pub fn get_last_selected_server(&self) -> String {
        self.settings.string("last-selected-server").to_string()
    }

    pub fn set_last_selected_server(&self, id: &str) {
        let _ = self.settings.set_string("last-selected-server", id);
    }

    pub fn get_last_selected_group(&self) -> String {
        self.settings.string("last-selected-group").to_string()
    }

    pub fn set_last_selected_group(&self, id: &str) {
        let _ = self.settings.set_string("last-selected-group", id);
    }

    pub fn get_search_history(&self) -> Vec<String> {
        self.settings
            .strv("search-history")
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn add_search_to_history(&self, query: &str) {
        let mut history = self.get_search_history();

        // Remove if already exists
        history.retain(|s| s != query);

        // Add to front
        history.insert(0, query.to_string());

        // Keep only last 10
        history.truncate(10);

        let strv: Vec<&str> = history.iter().map(|s| s.as_str()).collect();
        let _ = self.settings.set_strv("search-history", &strv);
    }

    // First run
    pub fn is_first_run(&self) -> bool {
        self.settings.boolean("first-run")
    }

    pub fn set_first_run_complete(&self) {
        let _ = self.settings.set_boolean("first-run", false);
    }

    /// Apply color scheme to libadwaita
    pub fn apply_color_scheme(&self) {
        let scheme_str = self.get_color_scheme();
        let style_manager = libadwaita::StyleManager::default();

        let scheme = match scheme_str.as_str() {
            "light" => libadwaita::ColorScheme::PreferLight,
            "dark" => libadwaita::ColorScheme::PreferDark,
            _ => libadwaita::ColorScheme::Default,
        };

        style_manager.set_color_scheme(scheme);
        tracing::info!("Applied color scheme: {}", scheme_str);
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self::new().expect("Failed to initialize settings")
    }
}

/// Save window state before closing
pub fn save_window_state(
    settings: &AppSettings,
    window: &adw::ApplicationWindow,
    sidebar: Option<&gtk4::Widget>,
) {
    // Save size (if not maximized)
    if !settings.get_window_maximized() {
        let width = window.width();
        let height = window.height();
        settings.set_window_size(width, height);
    }

    // Save maximized state
    let maximized = window.is_maximized();
    settings.set_window_maximized(maximized);

    // Save sidebar width if visible
    if let Some(sidebar) = sidebar {
        if sidebar.is_visible() {
            let width = sidebar.width();
            if width > 0 {
                settings.set_sidebar_width(width);
            }
        }
        settings.set_sidebar_visible(sidebar.is_visible());
    }

    tracing::info!("Window state saved");
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require GSettings to be installed
    // They are provided as documentation of the expected API

    #[test]
    fn test_color_scheme_default() {
        // settings.get_color_scheme() returns "default" initially
    }

    #[test]
    fn test_window_size_persistence() {
        // settings.set_window_size(1280, 800);
        // let (w, h) = settings.get_window_size();
        // assert_eq!(w, 1280);
        // assert_eq!(h, 800);
    }
}
