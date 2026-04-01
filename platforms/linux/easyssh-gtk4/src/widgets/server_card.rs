use gtk4::prelude::*;
use gtk4::glib;

use crate::models::{Server, ServerStatus};

pub struct ServerCard {
    widget: gtk4::Box,
    server_id: String,
}

impl ServerCard {
    pub fn new(server: &Server) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        box_.add_css_class("server-card");
        box_.set_margin_top(8);
        box_.set_margin_bottom(8);
        box_.set_margin_start(8);
        box_.set_margin_end(8);

        // Header with icon and name
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);

        let icon = gtk4::Image::from_icon_name("network-server-symbolic");
        icon.set_pixel_size(32);

        let name_label = gtk4::Label::new(Some(&server.name));
        name_label.add_css_class("title-4");
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_hexpand(true);

        let status = gtk4::Image::from_icon_name("circle-small-symbolic");
        let status_class = match server.status {
            ServerStatus::Connected => "status-connected",
            ServerStatus::Error => "status-error",
            _ => "status-unknown",
        };
        status.add_css_class(status_class);

        header.append(&icon);
        header.append(&name_label);
        header.append(&status);

        // Details
        let details = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        details.set_margin_start(40);

        let subtitle = gtk4::Label::new(Some(&format!(
            "{}@{}:{}",
            server.username, server.host, server.port
        )));
        subtitle.add_css_class("dim-label");
        subtitle.add_css_class("caption");
        subtitle.set_halign(gtk4::Align::Start);

        details.append(&subtitle);

        box_.append(&header);
        box_.append(&details);

        Self {
            widget: box_,
            server_id: server.id.clone(),
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }
}
