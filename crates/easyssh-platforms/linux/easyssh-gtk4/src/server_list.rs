use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;

use crate::models::{AuthType, Server, ServerStatus};

pub struct ServerList {
    widget: gtk4::Box,
    list_box: gtk4::ListBox,
    servers: RefCell<Vec<Server>>,
    selected_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
    connect_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
    edit_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
    delete_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
}

impl ServerList {
    pub fn new() -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        widget.add_css_class("server-list");

        // Header with count
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        header.set_margin_start(12);
        header.set_margin_end(12);
        header.set_margin_top(12);
        header.set_margin_bottom(6);

        let title = gtk4::Label::new(Some("Servers"));
        title.add_css_class("title-4");
        title.set_halign(gtk4::Align::Start);
        title.set_hexpand(true);
        header.append(&title);

        let count_label = gtk4::Label::new(Some("0"));
        count_label.add_css_class("dim-label");
        header.append(&count_label);

        widget.append(&header);

        // Separator
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        widget.append(&separator);

        // List box
        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.add_css_class("server-list");

        // Scrolled window
        let scrolled = gtk4::ScrolledWindow::builder()
            .child(&list_box)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .build();

        widget.append(&scrolled);

        let server_list = Self {
            widget,
            list_box,
            servers: RefCell::new(Vec::new()),
            selected_callback: RefCell::new(None),
            connect_callback: RefCell::new(None),
            edit_callback: RefCell::new(None),
            delete_callback: RefCell::new(None),
        };

        server_list.setup_signals();
        server_list
    }

    fn setup_signals(&self) {
        let selected_cb = self.selected_callback.clone();
        let connect_cb = self.connect_callback.clone();
        let edit_cb = self.edit_callback.clone();
        let delete_cb = self.delete_callback.clone();
        let servers = self.servers.clone();

        // Row selected
        self.list_box.connect_row_selected(move |_, row| {
            if let Some(ref callback) = *selected_cb.borrow() {
                if let Some(row) = row {
                    if let Some(server_id) = row.data::<String>("server-id") {
                        let id = server_id.as_ref();
                        if let Some(server) = servers.borrow().iter().find(|s| s.id == *id) {
                            callback(server.clone());
                        }
                    }
                }
            }
        });

        // Right-click context menu
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::ffi::GDK_BUTTON_SECONDARY as u32);

        let list_weak = self.list_box.downgrade();
        let servers_clone = servers.clone();
        let connect_cb_clone = connect_cb.clone();
        let edit_cb_clone = edit_cb.clone();
        let delete_cb_clone = delete_cb.clone();

        gesture.connect_pressed(move |gesture, _n, x, y| {
            if let Some(list) = list_weak.upgrade() {
                if let Some(row) = list.row_at_y(y as i32) {
                    list.select_row(Some(&row));

                    if let Some(server_id) = row.data::<String>("server-id") {
                        if let Some(server) = servers_clone
                            .borrow()
                            .iter()
                            .find(|s| s.id == *server_id.as_ref())
                        {
                            let menu = gio::Menu::new();
                            menu.append(Some("Connect"), Some("server.connect"));
                            menu.append(Some("Edit"), Some("server.edit"));
                            menu.append(Some("Delete"), Some("server.delete"));

                            let popover = gtk4::PopoverMenu::builder()
                                .menu_model(&menu)
                                .has_arrow(false)
                                .build();

                            // Position popover
                            let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
                            popover.set_pointing_to(&rect);
                            popover.set_parent(&list);

                            // Setup actions
                            let action_group = gio::SimpleActionGroup::new();

                            let server_connect = server.clone();
                            let connect_action = gio::SimpleAction::new("connect", None);
                            let connect_cb_inner = connect_cb_clone.clone();
                            connect_action.connect_activate(move |_, _| {
                                if let Some(ref cb) = *connect_cb_inner.borrow() {
                                    cb(server_connect.clone());
                                }
                            });

                            let server_edit = server.clone();
                            let edit_action = gio::SimpleAction::new("edit", None);
                            let edit_cb_inner = edit_cb_clone.clone();
                            edit_action.connect_activate(move |_, _| {
                                if let Some(ref cb) = *edit_cb_inner.borrow() {
                                    cb(server_edit.clone());
                                }
                            });

                            let server_delete = server.clone();
                            let delete_action = gio::SimpleAction::new("delete", None);
                            let delete_cb_inner = delete_cb_clone.clone();
                            delete_action.connect_activate(move |_, _| {
                                if let Some(ref cb) = *delete_cb_inner.borrow() {
                                    cb(server_delete.clone());
                                }
                            });

                            action_group.add_action(&connect_action);
                            action_group.add_action(&edit_action);
                            action_group.add_action(&delete_action);
                            list.insert_action_group("server", Some(&action_group));

                            popover.popup();
                        }
                    }
                }
            }
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });

        self.list_box.add_controller(gesture);
    }

    pub fn set_servers(&self, servers: Vec<Server>) {
        // Clear existing rows
        while let Some(row) = self.list_box.row_at_index(0) {
            self.list_box.remove(&row);
        }

        // Add new rows
        for server in &servers {
            let row = create_server_row(server);
            self.list_box.append(&row);
        }

        // Update count in header
        if let Some(header) = self.widget.first_child() {
            if let Some(hbox) = header.downcast_ref::<gtk4::Box>() {
                if let Some(count_label) = hbox.last_child() {
                    if let Some(label) = count_label.downcast_ref::<gtk4::Label>() {
                        label.set_text(&servers.len().to_string());
                    }
                }
            }
        }

        self.servers.replace(servers);
    }

    pub fn connect_server_selected<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.selected_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_server_connect<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.connect_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_server_edit<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.edit_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_server_delete<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.delete_callback.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

impl Clone for ServerList {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            list_box: self.list_box.clone(),
            servers: self.servers.clone(),
            selected_callback: RefCell::new(None),
            connect_callback: RefCell::new(None),
            edit_callback: RefCell::new(None),
            delete_callback: RefCell::new(None),
        }
    }
}

fn create_server_row(server: &Server) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_data("server-id", server.id.clone());

    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(10);
    hbox.set_margin_bottom(10);

    // Status icon
    let status_icon = gtk4::Image::from_icon_name(server.status.icon_name());
    status_icon.set_pixel_size(16);
    status_icon.add_css_class("dim-label");
    hbox.append(&status_icon);

    // Server info
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    vbox.set_hexpand(true);

    let name_label = gtk4::Label::new(Some(&server.name));
    name_label.set_halign(gtk4::Align::Start);
    name_label.add_css_class("heading");
    vbox.append(&name_label);

    let details = gtk4::Label::new(Some(&format!(
        "{}@{}:{}",
        server.username, server.host, server.port
    )));
    details.set_halign(gtk4::Align::Start);
    details.add_css_class("dim-label");
    details.add_css_class("caption");
    vbox.append(&details);

    hbox.append(&vbox);

    // Auth type icon
    let auth_icon_name = match server.auth_type {
        AuthType::Password => "dialog-password-symbolic",
        AuthType::Key => "key-symbolic",
        AuthType::Agent => "fingerprint-symbolic",
    };
    let auth_icon = gtk4::Image::from_icon_name(auth_icon_name);
    auth_icon.set_pixel_size(14);
    auth_icon.add_css_class("dim-label");
    auth_icon.set_tooltip_text(Some(match server.auth_type {
        AuthType::Password => "Password authentication",
        AuthType::Key => "SSH key authentication",
        AuthType::Agent => "SSH agent authentication",
    }));
    hbox.append(&auth_icon);

    row.set_child(Some(&hbox));
    row
}
