use gtk4::prelude::*;

use crate::models::{Server, ServerGroup};

pub struct ServerListView {
    list_box: gtk4::ListBox,
    servers: std::cell::RefCell<Vec<Server>>,
    groups: std::cell::RefCell<Vec<ServerGroup>>,
}

impl ServerListView {
    pub fn new() -> Self {
        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("navigation-sidebar");
        list_box.set_selection_mode(gtk4::SelectionMode::Single);

        // Placeholder row
        let placeholder = gtk4::Label::new(Some("No servers added yet"));
        placeholder.add_css_class("dim-label");
        placeholder.set_margin_top(16);
        placeholder.set_margin_bottom(16);
        list_box.set_placeholder(Some(&placeholder));

        Self {
            list_box,
            servers: std::cell::RefCell::new(Vec::new()),
            groups: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn set_servers(&self, servers: Vec<Server>) {
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Add server rows
        for server in &servers {
            let row = Self::create_server_row(server);
            self.list_box.append(&row);
        }

        self.servers.replace(servers);
    }

    pub fn set_groups(&self, groups: Vec<ServerGroup>) {
        self.groups.replace(groups);
    }

    fn create_server_row(server: &Server) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_.set_margin_start(12);
        box_.set_margin_end(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(8);

        // Status indicator
        let status = gtk4::Image::from_icon_name("circle-small-symbolic");
        status.add_css_class("success"); // or "warning", "error"
        status.set_pixel_size(16);

        let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let name = gtk4::Label::new(Some(&server.name));
        name.set_halign(gtk4::Align::Start);
        name.set_tooltip_text(Some(&server.name));

        let subtitle = gtk4::Label::new(Some(&format!(
            "{}@{}:{}",
            server.username, server.host, server.port
        )));
        subtitle.add_css_class("caption");
        subtitle.add_css_class("dim-label");
        subtitle.set_halign(gtk4::Align::Start);

        text_box.append(&name);
        text_box.append(&subtitle);

        box_.append(&status);
        box_.append(&text_box);

        row.set_child(Some(&box_));
        row
    }

    pub fn widget(&self) -> &gtk4::ListBox {
        &self.list_box
    }
}
