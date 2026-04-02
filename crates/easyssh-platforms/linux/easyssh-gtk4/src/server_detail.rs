use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;

use crate::models::{AuthType, Server, ServerStatus};

pub struct ServerDetail {
    widget: adw::StatusPage,
    content_box: gtk4::Box,
    server: RefCell<Option<Server>>,
    connect_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
    edit_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
    delete_callback: RefCell<Option<Box<dyn Fn(Server) + 'static>>>,
}

impl ServerDetail {
    pub fn new() -> Self {
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 24);
        content_box.set_margin_start(24);
        content_box.set_margin_end(24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);

        let widget = adw::StatusPage::builder()
            .icon_name("network-wired-disconnected-symbolic")
            .title("No Server Selected")
            .description("Select a server from the list to view details")
            .child(&content_box)
            .build();

        widget.set_vexpand(true);

        Self {
            widget,
            content_box,
            server: RefCell::new(None),
            connect_callback: RefCell::new(None),
            edit_callback: RefCell::new(None),
            delete_callback: RefCell::new(None),
        }
    }

    pub fn set_server(&self, server: Option<&Server>) {
        // Clear existing content
        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }

        if let Some(server) = server {
            self.server.replace(Some(server.clone()));
            self.build_server_details(server);
        } else {
            self.server.replace(None);
            self.build_empty_state();
        }
    }

    fn build_empty_state(&self) {
        self.widget.set_icon_name(Some("network-wired-disconnected-symbolic"));
        self.widget.set_title("No Server Selected");
        self.widget.set_description(Some("Select a server from the list to view details"));
    }

    fn build_server_details(&self, server: &Server) {
        // Update status page
        self.widget.set_icon_name(None);
        self.widget.set_title(&server.name);
        self.widget.set_description(None);

        // Connection info group
        let info_group = adw::PreferencesGroup::builder()
            .title("Connection Information")
            .build();

        // Host row
        let host_row = adw::ActionRow::builder()
            .title("Host")
            .subtitle(&format!("{}", server.host))
            .build();
        host_row.add_prefix(&gtk4::Image::from_icon_name("network-server-symbolic"));
        host_row.set_activatable(false);
        info_group.add(&host_row);

        // Port row
        let port_row = adw::ActionRow::builder()
            .title("Port")
            .subtitle(&server.port.to_string())
            .build();
        port_row.add_prefix(&gtk4::Image::from_icon_name("network-receive-symbolic"));
        port_row.set_activatable(false);
        info_group.add(&port_row);

        // Username row
        let user_row = adw::ActionRow::builder()
            .title("Username")
            .subtitle(&server.username)
            .build();
        user_row.add_prefix(&gtk4::Image::from_icon_name("avatar-default-symbolic"));
        user_row.set_activatable(false);
        info_group.add(&user_row);

        self.content_box.append(&info_group);

        // Authentication group
        let auth_group = adw::PreferencesGroup::builder()
            .title("Authentication")
            .build();

        let (auth_type_str, auth_detail) = match server.auth_type {
            AuthType::Password => ("Password", "Stored in keychain"),
            AuthType::Key => (
                "SSH Key",
                server
                    .identity_file
                    .as_deref()
                    .unwrap_or("~/.ssh/id_rsa"),
            ),
            AuthType::Agent => ("SSH Agent", "System SSH agent"),
        };

        let auth_row = adw::ActionRow::builder()
            .title("Method")
            .subtitle(&format!("{} - {}", auth_type_str, auth_detail))
            .build();
        let auth_icon_name = match server.auth_type {
            AuthType::Password => "dialog-password-symbolic",
            AuthType::Key => "key-symbolic",
            AuthType::Agent => "fingerprint-symbolic",
        };
        auth_row.add_prefix(&gtk4::Image::from_icon_name(auth_icon_name));
        auth_row.set_activatable(false);
        auth_group.add(&auth_row);

        self.content_box.append(&auth_group);

        // Status group
        let status_group = adw::PreferencesGroup::builder()
            .title("Status")
            .build();

        let status_str = match server.status {
            ServerStatus::Connected => "Connected",
            ServerStatus::Disconnected => "Disconnected",
            ServerStatus::Error => "Error",
            ServerStatus::Unknown => "Unknown",
        };
        let status_row = adw::ActionRow::builder()
            .title("Connection Status")
            .subtitle(status_str)
            .build();
        let status_icon_name = server.status.icon_name();
        let status_icon = gtk4::Image::from_icon_name(status_icon_name);
        match server.status {
            ServerStatus::Connected => status_icon.add_css_class("success"),
            ServerStatus::Error => status_icon.add_css_class("error"),
            _ => status_icon.add_css_class("dim-label"),
        }
        status_row.add_prefix(&status_icon);
        status_row.set_activatable(false);
        status_group.add(&status_row);

        self.content_box.append(&status_group);

        // Actions group
        let actions_group = adw::PreferencesGroup::builder()
            .title("Actions")
            .build();

        // Connect button
        let connect_button = gtk4::Button::builder()
            .label("Connect")
            .css_classes(vec!["suggested-action", "pill"])
            .halign(gtk4::Align::Center)
            .build();
        connect_button.set_icon_name("network-connect-symbolic");

        let server_connect = server.clone();
        let connect_cb = self.connect_callback.clone();
        connect_button.connect_clicked(move |_| {
            if let Some(ref callback) = *connect_cb.borrow() {
                callback(server_connect.clone());
            }
        });

        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        button_box.set_halign(gtk4::Align::Center);
        button_box.append(&connect_button);

        // Edit and Delete buttons
        let edit_button = gtk4::Button::builder()
            .label("Edit")
            .css_classes(vec!["pill"])
            .build();
        edit_button.set_icon_name("document-edit-symbolic");

        let server_edit = server.clone();
        let edit_cb = self.edit_callback.clone();
        edit_button.connect_clicked(move |_| {
            if let Some(ref callback) = *edit_cb.borrow() {
                callback(server_edit.clone());
            }
        });

        let delete_button = gtk4::Button::builder()
            .label("Delete")
            .css_classes(vec!["destructive-action", "pill"])
            .build();
        delete_button.set_icon_name("user-trash-symbolic");

        let server_delete = server.clone();
        let delete_cb = self.delete_callback.clone();
        delete_button.connect_clicked(move |_| {
            if let Some(ref callback) = *delete_cb.borrow() {
                callback(server_delete.clone());
            }
        });

        button_box.append(&edit_button);
        button_box.append(&delete_button);

        actions_group.add(&button_box);
        self.content_box.append(&actions_group);
    }

    pub fn connect_connect_clicked<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.connect_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_edit_clicked<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.edit_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_delete_clicked<F>(&self, callback: F)
    where
        F: Fn(Server) + 'static,
    {
        self.delete_callback.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &adw::StatusPage {
        &self.widget
    }
}

impl Clone for ServerDetail {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            content_box: self.content_box.clone(),
            server: RefCell::new(None),
            connect_callback: RefCell::new(None),
            edit_callback: RefCell::new(None),
            delete_callback: RefCell::new(None),
        }
    }
}
