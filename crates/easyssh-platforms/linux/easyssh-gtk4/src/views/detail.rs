use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;

use crate::app::AppViewModel;
use crate::models::{Server, ServerStatus};

pub struct ServerDetailView {
    widget: gtk4::Box,
    view_model: Arc<Mutex<AppViewModel>>,
    server: RefCell<Option<Server>>,

    // Widgets for dynamic updates
    title_label: gtk4::Label,
    subtitle_label: gtk4::Label,
    status_label: gtk4::Label,
    connect_btn: gtk4::Button,
    favorite_btn: gtk4::Button,
    delete_btn: gtk4::Button,
    auth_row: adw::ActionRow,
    port_row: adw::ActionRow,
    host_row: adw::ActionRow,
    username_row: adw::ActionRow,

    // Callbacks
    connect_callback: RefCell<Option<Box<dyn Fn()>>>,
    delete_callback: RefCell<Option<Box<dyn Fn()>>>,
    favorite_callback: RefCell<Option<Box<dyn Fn()>>>,
    has_saved_password: RefCell<bool>,
}

impl ServerDetailView {
    pub fn new(view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_.set_vexpand(true);
        box_.set_hexpand(true);

        // Header area
        let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
        header_box.set_margin(24);

        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_pixel_size(48);
        icon.add_css_class("accent");

        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        title_box.set_hexpand(true);

        let title_label = gtk4::Label::new(None);
        title_label.add_css_class("title-1");
        title_label.set_halign(gtk4::Align::Start);

        let subtitle_label = gtk4::Label::new(None);
        subtitle_label.add_css_class("dim-label");
        subtitle_label.set_halign(gtk4::Align::Start);

        title_box.append(&title_label);
        title_box.append(&subtitle_label);

        let status_label = gtk4::Label::new(Some("● Unknown"));
        status_label.add_css_class("dim-label");

        header_box.append(&icon);
        header_box.append(&title_box);
        header_box.append(&status_label);

        // Action buttons
        let action_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        action_box.set_margin(24);

        let connect_btn = gtk4::Button::with_label("Connect (Terminal)");
        connect_btn.add_css_class("suggested-action");
        connect_btn.add_css_class("pill");

        let favorite_btn = gtk4::Button::from_icon_name("star-outline-symbolic");
        favorite_btn.set_tooltip_text(Some("Add to Favorites"));
        favorite_btn.add_css_class("favorite-button");

        let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
        edit_btn.set_tooltip_text(Some("Edit Server"));

        let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
        delete_btn.set_tooltip_text(Some("Delete Server"));
        delete_btn.add_css_class("destructive-action");

        action_box.append(&connect_btn);
        action_box.append(&favorite_btn);
        action_box.append(&edit_btn);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        action_box.append(&spacer);

        action_box.append(&delete_btn);

        // Info card
        let info_card = adw::PreferencesGroup::new();
        info_card.set_title("Connection Details");
        info_card.set_margin(24);

        let host_row = adw::ActionRow::new();
        host_row.set_title("Host");
        host_row.set_icon_name("network-server-symbolic");
        info_card.add(&host_row);

        let port_row = adw::ActionRow::new();
        port_row.set_title("Port");
        port_row.set_icon_name("preferences-system-network-symbolic");
        info_card.add(&port_row);

        let username_row = adw::ActionRow::new();
        username_row.set_title("Username");
        username_row.set_icon_name("avatar-default-symbolic");
        info_card.add(&username_row);

        let auth_row = adw::ActionRow::new();
        auth_row.set_title("Authentication");
        auth_row.set_icon_name("dialog-password-symbolic");
        info_card.add(&auth_row);

        // Saved password indicator
        let password_row = adw::ActionRow::new();
        password_row.set_title("Saved Password");
        password_row.set_subtitle("No saved password");
        password_row.set_icon_name("dialog-password-symbolic");
        info_card.add(&password_row);

        // Tags section (placeholder)
        let tags_card = adw::PreferencesGroup::new();
        tags_card.set_title("Tags");
        tags_card.set_description(Some("Organize your server with tags"));
        tags_card.set_margin(24);

        let no_tags_label = gtk4::Label::new(Some("No tags added"));
        no_tags_label.add_css_class("dim-label");
        no_tags_label.set_margin(8);
        tags_card.add(&no_tags_label);

        // Last used section
        let usage_card = adw::PreferencesGroup::new();
        usage_card.set_title("Usage");
        usage_card.set_margin(24);

        let last_used_row = adw::ActionRow::new();
        last_used_row.set_title("Last Connected");
        last_used_row.set_subtitle("Never");
        last_used_row.set_icon_name("clock-symbolic");
        usage_card.add(&last_used_row);

        let sessions_row = adw::ActionRow::new();
        sessions_row.set_title("Active Sessions");
        sessions_row.set_subtitle("0");
        sessions_row.set_icon_name("network-transmit-receive-symbolic");
        usage_card.add(&sessions_row);

        // Scroll container for content
        let scroll = gtk4::ScrolledWindow::new();
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content_box.append(&header_box);
        content_box.append(&action_box);
        content_box.append(&info_card);
        content_box.append(&tags_card);
        content_box.append(&usage_card);
        scroll.set_child(Some(&content_box));
        scroll.set_vexpand(true);

        box_.append(&scroll);

        let view = Self {
            widget: box_,
            view_model,
            server: RefCell::new(None),
            title_label,
            subtitle_label,
            status_label,
            connect_btn,
            favorite_btn,
            delete_btn,
            auth_row,
            port_row,
            host_row,
            username_row,
            connect_callback: RefCell::new(None),
            delete_callback: RefCell::new(None),
            favorite_callback: RefCell::new(None),
            has_saved_password: RefCell::new(false),
        };

        // Connect signals
        view.connect_btn
            .connect_clicked(glib::clone!(@weak view as v => move |_| {
                if let Some(ref cb) = *v.connect_callback.borrow() {
                    cb();
                }
            }));

        view.delete_btn
            .connect_clicked(glib::clone!(@weak view as v => move |_| {
                if let Some(ref cb) = *v.delete_callback.borrow() {
                    cb();
                }
            }));

        view.favorite_btn
            .connect_clicked(glib::clone!(@weak view as v => move |_| {
                if let Some(ref cb) = *v.favorite_callback.borrow() {
                    cb();
                }
            }));

        view
    }

    pub fn set_server(&self, server: Server) {
        self.title_label.set_text(&server.name);
        self.subtitle_label
            .set_text(&format!("{}@{}", server.username, server.host));

        // Status
        let (status_text, status_class) = match server.status {
            ServerStatus::Connected => ("● Connected", "status-connected"),
            ServerStatus::Error => ("● Error", "status-error"),
            ServerStatus::Disconnected => ("● Disconnected", "status-disconnected"),
            _ => ("● Unknown", "status-unknown"),
        };
        self.status_label.set_text(status_text);
        self.status_label.set_css_classes(&[status_class]);

        // Connection details
        self.host_row.set_subtitle(&server.host);
        self.port_row.set_subtitle(&server.port.to_string());
        self.username_row.set_subtitle(&server.username);

        let auth_text = match server.auth_type {
            crate::models::AuthType::Password => "Password",
            crate::models::AuthType::Key => "SSH Key",
            crate::models::AuthType::Agent => "SSH Agent",
        };
        self.auth_row.set_subtitle(auth_text);

        self.server.replace(Some(server));
    }

    pub fn set_has_saved_password(&self, has_saved: bool) {
        self.has_saved_password.replace(has_saved);
        if has_saved {
            self.auth_row.set_subtitle("Password (saved in keyring)");
        }
    }

    pub fn set_is_favorite(&self, is_favorite: bool) {
        let icon_name = if is_favorite {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        };
        self.favorite_btn.set_icon_name(icon_name);
    }

    pub fn connect_connect_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.connect_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_delete_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.delete_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_favorite_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.favorite_callback.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}
