use gtk4::glib;
use gtk4::prelude::*;

use crate::models::{Server, ServerStatus};

pub struct ServerListRow {
    widget: gtk4::ListBoxRow,
    server_id: String,
}

impl ServerListRow {
    pub fn new(server: &Server, is_favorite: bool) -> Self {
        let row = gtk4::ListBoxRow::new();
        row.add_css_class("server-row");
        row.set_name(&server.id);

        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        box_.set_margin_start(12);
        box_.set_margin_end(12);
        box_.set_margin_top(8);
        box_.set_margin_bottom(8);

        // Status indicator
        let status_icon = match server.status {
            ServerStatus::Connected => "●",
            _ => "○",
        };
        let status = gtk4::Label::new(Some(status_icon));
        let status_class = match server.status {
            ServerStatus::Connected => "status-connected",
            ServerStatus::Error => "status-error",
            _ => "status-unknown",
        };
        status.add_css_class(status_class);
        status.set_pixel_size(10);

        // Text content
        let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let name_text = if is_favorite {
            format!("★ {}", server.name)
        } else {
            server.name.clone()
        };

        let name = gtk4::Label::new(Some(&name_text));
        name.set_halign(gtk4::Align::Start);
        name.set_tooltip_text(Some(&server.name));
        if is_favorite {
            name.add_css_class("favorite");
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

        Self {
            widget: row,
            server_id: server.id.clone(),
        }
    }

    pub fn widget(&self) -> &gtk4::ListBoxRow {
        &self.widget
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }
}
