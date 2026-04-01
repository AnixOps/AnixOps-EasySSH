use gtk4::prelude::*;

pub struct EmptyView {
    widget: gtk4::Box,
}

impl EmptyView {
    pub fn new() -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        box_.set_valign(gtk4::Align::Center);
        box_.set_halign(gtk4::Align::Center);
        box_.add_css_class("empty-state");

        let icon = gtk4::Image::from_icon_name("utilities-terminal-symbolic");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_pixel_size(96);
        icon.set_opacity(0.3);
        icon.add_css_class("empty-state-icon");

        let title = gtk4::Label::new(Some("EasySSH"));
        title.add_css_class("title-1");
        title.add_css_class("dim-label");

        let subtitle = gtk4::Label::new(Some("Select a server or add a new one to get started"));
        subtitle.add_css_class("dim-label");
        subtitle.add_css_class("body");

        let hint = gtk4::Label::new(Some("Press + to add a server"));
        hint.add_css_class("dim-label");
        hint.add_css_class("caption");
        hint.set_margin_top(8);

        box_.append(&icon);
        box_.append(&title);
        box_.append(&subtitle);
        box_.append(&hint);

        Self { widget: box_ }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}