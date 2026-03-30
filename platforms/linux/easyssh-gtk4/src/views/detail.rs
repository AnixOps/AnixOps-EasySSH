use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

use crate::models::{Server, ServerStatus};

pub struct ServerDetailView {
    widget: gtk4::Box,
    core_state: Arc<Mutex<easyssh_core::AppState>>,
    server: std::cell::RefCell<Option<Server>>,
}

impl ServerDetailView {
    pub fn new(core_state: Arc<Mutex<easyssh_core::AppState>>) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        box_.set_margin_top(24);
        box_.set_margin_bottom(24);
        box_.set_margin_start(24);
        box_.set_margin_end(24);

        // Header
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
        header.set_valign(gtk4::Align::Start);

        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_pixel_size(48);
        icon.add_css_class("accent");

        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        title_box.set_hexpand(true);

        let title = gtk4::Label::new(None);
        title.add_css_class("title-1");
        title.set_halign(gtk4::Align::Start);

        let subtitle = gtk4::Label::new(None);
        subtitle.add_css_class("dim-label");
        subtitle.set_halign(gtk4::Align::Start);

        title_box.append(&title);
        title_box.append(&subtitle);

        let status_badge = gtk4::Label::new(Some("● Unknown"));
        status_badge.add_css_class("dim-label");

        header.append(&icon);
        header.append(&title_box);
        header.append(&status_badge);

        // Actions
        let action_group = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let connect_btn = gtk4::Button::with_label("Connect (Terminal)");
        connect_btn.add_css_class("suggested-action");
        connect_btn.add_css_class("pill");

        let connect_embedded_btn = gtk4::Button::with_label("Connect (Embedded)");
        connect_embedded_btn.set_sensitive(false); // TODO: Embedded terminal
        connect_embedded_btn.add_css_class("pill");

        let edit_btn = gtk4::Button::from_icon_name("document-edit-symbolic");
        edit_btn.set_tooltip_text(Some("Edit Server"));

        action_group.append(&connect_btn);
        action_group.append(&connect_embedded_btn);
        action_group.append(&edit_btn);

        // Info card
        let card = adw::PreferencesGroup::new();
        card.set_title("Connection Details");

        let auth_row = adw::ActionRow::new();
        auth_row.set_title("Authentication");

        let port_row = adw::ActionRow::new();
        port_row.set_title("Port");

        card.add(&auth_row);
        card.add(&port_row);

        box_.append(&header);
        box_.append(&action_group);
        box_.append(&card);

        let view = Self {
            widget: box_,
            core_state,
            server: std::cell::RefCell::new(None),
        };

        // Connect signals
        connect_btn.connect_clicked(glib::clone!(@strong view => move |_| {
            if let Some(server) = view.server.borrow().as_ref() {
                view.connect_native(server);
            }
        }));

        view
    }

    pub fn set_server(&self, server: Server) {
        // Update UI with server info
        self.server.replace(Some(server));
    }

    fn connect_native(&self, server: &Server) {
        let state = self.core_state.lock().unwrap();
        if let Err(e) = easyssh_core::connect_server(&state, &server.id) {
            tracing::error!("Failed to connect: {}", e);
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

use gtk4::glib;
