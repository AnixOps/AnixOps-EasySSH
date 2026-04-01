use gtk4::prelude::*;
use libadwaita::prelude::*;

/// Connection Pool Monitor Widget
/// Displays real-time connection pool statistics
pub struct ConnectionPoolMonitor {
    widget: gtk4::Box,

    // Stats labels
    total_pools_label: gtk4::Label,
    global_connections_label: gtk4::Label,
    rate_limited_label: gtk4::Label,
    compression_ratio_label: gtk4::Label,
    session_count_label: gtk4::Label,

    // Session list
    session_list: gtk4::ListBox,
}

impl ConnectionPoolMonitor {
    pub fn new() -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);

        // Header
        let header = gtk4::Label::new(Some("Connection Pool Status"));
        header.add_css_class("title-2");
        widget.append(&header);

        // Stats grid
        let stats_frame = gtk4::Frame::new(Some("Statistics"));
        let stats_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        stats_box.set_margin_top(12);
        stats_box.set_margin_bottom(12);
        stats_box.set_margin_start(12);
        stats_box.set_margin_end(12);

        let total_pools_label = Self::create_stat_row(&stats_box, "Active Pools:");
        let global_connections_label = Self::create_stat_row(&stats_box, "Global Connections:");
        let rate_limited_label = Self::create_stat_row(&stats_box, "Rate Limited:");
        let compression_ratio_label = Self::create_stat_row(&stats_box, "Compression Ratio:");
        let session_count_label = Self::create_stat_row(&stats_box, "Stored Sessions:");

        stats_frame.set_child(Some(&stats_box));
        widget.append(&stats_frame);

        // Session states list
        let sessions_frame = gtk4::Frame::new(Some("Session States"));
        let session_list = gtk4::ListBox::new();
        session_list.set_selection_mode(gtk4::SelectionMode::None);
        session_list.add_css_class("boxed-list");

        // Placeholder row
        let placeholder = gtk4::Label::new(Some("No active sessions"));
        placeholder.set_opacity(0.5);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        session_list.append(&placeholder);

        let sessions_scroll = gtk4::ScrolledWindow::new();
        sessions_scroll.set_child(Some(&session_list));
        sessions_scroll.set_vexpand(true);
        sessions_scroll.set_min_content_height(200);

        sessions_frame.set_child(Some(&sessions_scroll));
        widget.append(&sessions_frame);

        Self {
            widget,
            total_pools_label,
            global_connections_label,
            rate_limited_label,
            compression_ratio_label,
            session_count_label,
            session_list,
        }
    }

    fn create_stat_row(parent: &gtk4::Box, label_text: &str) -> gtk4::Label {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        row.set_hexpand(true);

        let label = gtk4::Label::new(Some(label_text));
        label.set_halign(gtk4::Align::Start);
        label.set_hexpand(true);
        row.append(&label);

        let value = gtk4::Label::new(Some("-"));
        value.set_halign(gtk4::Align::End);
        value.add_css_class("monospace");
        row.append(&value);

        parent.append(&row);
        value
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    /// Update statistics display (called from external timer)
    pub fn update_stats(
        &self,
        total_pools: usize,
        global_connections: usize,
        max_global: usize,
        rate_limited: usize,
        compression_ratio: f64,
        session_count: usize,
    ) {
        self.total_pools_label.set_text(&total_pools.to_string());
        self.global_connections_label
            .set_text(&format!("{}/{}", global_connections, max_global));
        self.rate_limited_label.set_text(&rate_limited.to_string());
        self.compression_ratio_label
            .set_text(&format!("{:.1}%", compression_ratio));
        self.session_count_label
            .set_text(&session_count.to_string());
    }

    /// Clear session list
    pub fn clear_sessions(&self) {
        while let Some(child) = self.session_list.first_child() {
            self.session_list.remove(&child);
        }

        let placeholder = gtk4::Label::new(Some("No active sessions"));
        placeholder.set_opacity(0.5);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        self.session_list.append(&placeholder);
    }

    /// Add a session row to the list
    pub fn add_session(&self, session_id: &str, state_text: &str, css_class: &str) {
        // Remove placeholder if exists
        if let Some(first) = self.session_list.first_child() {
            if let Some(label) = first.downcast_ref::<gtk4::Label>() {
                if label.text() == "No active sessions" {
                    self.session_list.remove(&first);
                }
            }
        }

        let row = gtk4::ListBoxRow::new();
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        // Session ID (truncated)
        let id_text = if session_id.len() > 11 {
            format!("{}...", &session_id[..8])
        } else {
            session_id.to_string()
        };
        let id_label = gtk4::Label::new(Some(&id_text));
        id_label.set_halign(gtk4::Align::Start);
        id_label.set_hexpand(true);
        id_label.add_css_class("monospace");
        hbox.append(&id_label);

        // State label
        let state_label = gtk4::Label::new(Some(state_text));
        state_label.set_halign(gtk4::Align::End);
        if !css_class.is_empty() {
            state_label.add_css_class(css_class);
        }
        hbox.append(&state_label);

        row.set_child(Some(&hbox));
        self.session_list.append(&row);
    }
}
