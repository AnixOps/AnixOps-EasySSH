use gtk4::prelude::*;
use gtk4::glib;
use std::cell::RefCell;

use crate::models::{Server, ServerGroup};

pub struct ServerListView {
    list_box: gtk4::ListBox,
    search_entry: RefCell<Option<gtk4::SearchEntry>>,
    servers: RefCell<Vec<Server>>,
    groups: RefCell<Vec<ServerGroup>>,
    favorites: RefCell<Vec<String>>,
    selection_callback: RefCell<Option<Box<dyn Fn(Server)>>>,
    filter_text: RefCell<String>,
    selected_group: RefCell<Option<String>>,
}

impl ServerListView {
    pub fn new() -> Self {
        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("server-list");
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.set_show_separators(false);

        // Placeholder
        let placeholder = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        placeholder.set_valign(gtk4::Align::Center);
        placeholder.set_halign(gtk4::Align::Center);

        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_pixel_size(64);
        icon.set_opacity(0.3);
        icon.add_css_class("empty-state-icon");

        let label = gtk4::Label::new(Some("No servers yet"));
        label.add_css_class("dim-label");
        label.add_css_class("title-3");

        let hint = gtk4::Label::new(Some("Add a server to get started"));
        hint.add_css_class("dim-label");
        hint.add_css_class("caption");

        placeholder.append(&icon);
        placeholder.append(&label);
        placeholder.append(&hint);

        list_box.set_placeholder(Some(&placeholder));

        let view = Self {
            list_box,
            search_entry: RefCell::new(None),
            servers: RefCell::new(Vec::new()),
            groups: RefCell::new(Vec::new()),
            favorites: RefCell::new(Vec::new()),
            selection_callback: RefCell::new(None),
            filter_text: RefCell::new(String::new()),
            selected_group: RefCell::new(None),
        };

        // Connect row selection
        view.list_box.connect_row_selected(glib::clone!(@weak view as v => move |_, row| {
            if let Some(row) = row {
                if let Some(server) = v.get_server_for_row(row) {
                    if let Some(ref cb) = *v.selection_callback.borrow() {
                        cb(server);
                    }
                }
            }
        }));

        view
    }

    pub fn set_servers(&self, servers: Vec<Server>) {
        self.servers.replace(servers);
        self.refresh_list();
    }

    pub fn set_favorites(&self, favorites: &[String]) {
        self.favorites.replace(favorites.to_vec());
        self.refresh_list();
    }

    pub fn set_groups(&self, groups: Vec<ServerGroup>) {
        self.groups.replace(groups);
    }

    pub fn set_filter(&self, text: &str) {
        self.filter_text.replace(text.to_string());
        self.refresh_list();
    }

    pub fn connect_selection_changed<F: Fn(Server) + 'static>(&self, callback: F) {
        self.selection_callback.replace(Some(Box::new(callback)));
    }

    fn refresh_list(&self) {
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let servers = self.servers.borrow();
        let filter = self.filter_text.borrow();
        let favorites = self.favorites.borrow();
        let selected_group = self.selected_group.borrow();

        // Build group mapping
        let mut ungrouped: Vec<&Server> = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<&Server>> = std::collections::HashMap::new();

        for server in servers.iter() {
            // Search filter
            if !filter.is_empty() {
                let f = filter.to_lowercase();
                if !server.name.to_lowercase().contains(&f) &&
                   !server.host.to_lowercase().contains(&f) &&
                   !server.username.to_lowercase().contains(&f) {
                    continue;
                }
            }

            // Group filter
            if let Some(ref group_id) = selected_group.as_ref() {
                if group_id == "__favorites__" {
                    if !favorites.contains(&server.id) {
                        continue;
                    }
                } else if group_id == "__ungrouped__" {
                    if server.group_id.is_some() {
                        continue;
                    }
                } else if let Some(ref sid) = server.group_id {
                    if sid != *group_id {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(ref group_id) = server.group_id {
                grouped.entry(group_id.clone()).or_default().push(server);
            } else {
                ungrouped.push(server);
            }
        }

        // Add group headers and servers
        let groups = self.groups.borrow();

        // First, add servers with groups
        for group in groups.iter() {
            if let Some(servers_in_group) = grouped.get(&group.id) {
                if !servers_in_group.is_empty() {
                    // Group header
                    let header = self.create_group_header(&group.name, &group.id);
                    self.list_box.append(&header);

                    // Servers in group
                    for server in servers_in_group {
                        let row = self.create_server_row(server, favorites.contains(&server.id));
                        self.list_box.append(&row);
                    }
                }
            }
        }

        // Add favorites section if there are any
        let favorite_servers: Vec<&Server> = servers.iter()
            .filter(|s| favorites.contains(&s.id) && s.group_id.is_none())
            .filter(|s| {
                if filter.is_empty() { return true; }
                let f = filter.to_lowercase();
                s.name.to_lowercase().contains(&f) ||
                s.host.to_lowercase().contains(&f)
            })
            .collect();

        if !favorite_servers.is_empty() {
            let fav_header = self.create_special_header("★ Favorites", "__favorites__");
            self.list_box.append(&fav_header);

            for server in favorite_servers {
                let row = self.create_server_row(server, true);
                self.list_box.append(&row);
            }
        }

        // Add ungrouped servers
        if !ungrouped.is_empty() {
            let ungrouped_header = self.create_special_header("📡 No Group", "__ungrouped__");
            self.list_box.append(&ungrouped_header);

            for server in ungrouped {
                let row = self.create_server_row(server, favorites.contains(&server.id));
                self.list_box.append(&row);
            }
        }
    }

    fn create_group_header(&self, name: &str, id: &str) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        row.set_selectable(false);
        row.add_css_class("group-header");

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        box_.set_margin_start(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(4);

        let icon = gtk4::Image::from_icon_name("folder-symbolic");
        icon.set_pixel_size(16);

        let label = gtk4::Label::new(Some(name));
        label.add_css_class("caption");
        label.set_halign(gtk4::Align::Start);

        box_.append(&icon);
        box_.append(&label);

        row.set_child(Some(&box_));

        // Click to filter by group
        let id = id.to_string();
        let controller = gtk4::GestureClick::new();
        controller.connect_pressed(glib::clone!(@weak self as view => move |_, _, _, _| {
            if view.selected_group.borrow().as_ref() == Some(&id) {
                view.selected_group.replace(None);
            } else {
                view.selected_group.replace(Some(id.clone()));
            }
            view.refresh_list();
        }));
        row.add_controller(controller);

        row
    }

    fn create_special_header(&self, name: &str, id: &str) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        row.set_selectable(false);

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        box_.set_margin_start(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(4);

        let label = gtk4::Label::new(Some(name));
        label.add_css_class("caption");
        label.add_css_class("dim-label");
        label.set_halign(gtk4::Align::Start);

        box_.append(&label);

        row.set_child(Some(&box_));

        // Click to filter
        let id = id.to_string();
        let controller = gtk4::GestureClick::new();
        controller.connect_pressed(glib::clone!(@weak self as view => move |_, _, _, _| {
            if view.selected_group.borrow().as_ref() == Some(&id) {
                view.selected_group.replace(None);
            } else {
                view.selected_group.replace(Some(id.clone()));
            }
            view.refresh_list();
        }));
        row.add_controller(controller);

        row
    }

    fn create_server_row(&self, server: &Server, is_favorite: bool) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::new();
        row.add_css_class("server-row");

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_.set_margin_start(12);
        box_.set_margin_end(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(8);

        // Status indicator
        let status_icon = match server.status {
            crate::models::ServerStatus::Connected => "●",
            _ => "○",
        };
        let status = gtk4::Label::new(Some(status_icon));
        let status_class = match server.status {
            crate::models::ServerStatus::Connected => "status-connected",
            crate::models::ServerStatus::Error => "status-error",
            _ => "status-unknown",
        };
        status.add_css_class(status_class);
        status.set_pixel_size(12);

        // Text content
        let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let name = gtk4::Label::new(Some(&server.name));
        name.set_halign(gtk4::Align::Start);
        name.set_tooltip_text(Some(&server.name));
        if is_favorite {
            name.set_markup(&format!("★ {}", glib::markup_escape_text(&server.name)));
        }

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

        // Store server ID as data
        row.set_name(&server.id);

        row
    }

    fn get_server_for_row(&self, row: &gtk4::ListBoxRow) -> Option<Server> {
        let id = row.name()?;
        self.servers.borrow().iter().find(|s| s.id == id).cloned()
    }

    pub fn widget(&self) -> &gtk4::ListBox {
        &self.list_box
    }
}
