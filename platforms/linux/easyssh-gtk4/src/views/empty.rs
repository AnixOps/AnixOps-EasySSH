use gtk4::prelude::*;

pub struct EmptyView {
    widget: gtk4::Box,
}

impl EmptyView {
    pub fn new() -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        box_.set_valign(gtk4::Align::Center);
        box_.set_halign(gtk4::Align::Center);

        let icon = gtk4::Image::from_icon_name("utilities-terminal-symbolic");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_pixel_size(64);
        icon.set_opacity(0.5);

        let title = gtk4::Label::new(Some("Select a Server"));
        title.add_css_class("title-2");

        let subtitle = gtk4::Label::new(Some("Choose a server from the sidebar to connect"));
        subtitle.add_css_class("dim-label");

        box_.append(&icon);
        box_.append(&title);
        box_.append(&subtitle);

        Self { widget: box_ }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}
