use gtk4::prelude::*;
use libadwaita::prelude::*;

pub fn show_password_dialog<F>(parent: &adw::ApplicationWindow, server_name: &str, callback: F)
where
    F: FnOnce(String) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("Enter Password")
        .content_width(400)
        .content_height(300)
        .build();

    let toolbar_view = adw::ToolbarView::new();

    // Header bar
    let header = adw::HeaderBar::new();
    header.add_css_class("flat");

    let cancel_button = gtk4::Button::builder()
        .label("Cancel")
        .build();
    header.pack_start(&cancel_button);

    let connect_button = gtk4::Button::builder()
        .label("Connect")
        .css_classes(vec!["suggested-action"])
        .build();
    header.pack_end(&connect_button);

    toolbar_view.add_top_bar(&header);

    // Content
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    // Icon
    let icon = gtk4::Image::from_icon_name("dialog-password-symbolic");
    icon.set_pixel_size(64);
    icon.set_margin_bottom(12);
    content.append(&icon);

    // Title
    let title = gtk4::Label::builder()
        .label(&format!("Connect to {}", server_name))
        .css_classes(vec!["title-2"])
        .build();
    content.append(&title);

    // Password field
    let group = adw::PreferencesGroup::new();
    let password_entry = adw::PasswordEntryRow::builder()
        .title("Password")
        .show_apply_button(false)
        .build();
    group.add(&password_entry);
    content.append(&group);

    // Remember checkbox
    let remember_check = gtk4::CheckButton::builder()
        .label("Remember password in keychain")
        .active(true)
        .margin_top(12)
        .build();
    content.append(&remember_check);

    toolbar_view.set_content(Some(&content));
    dialog.set_child(Some(&toolbar_view));

    // Cancel button
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    // Connect button
    let dialog_weak = dialog.downgrade();
    connect_button.connect_clicked(glib::clone!(@weak password_entry => move |_| {
        let password = password_entry.text().to_string();

        if password.is_empty() {
            return;
        }

        callback(password);

        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    }));

    // Enter key handler
    password_entry.connect_entry_activated(glib::clone!(@weak connect_button => move |_| {
        connect_button.activate();
    }));

    dialog.present(parent);
}
