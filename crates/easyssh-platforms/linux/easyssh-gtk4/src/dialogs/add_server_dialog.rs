use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::models::{AuthType, Server, ServerStatus};

pub fn show_add_server_dialog<F>(parent: &adw::ApplicationWindow, callback: F)
where
    F: FnOnce(Server) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("Add Server")
        .content_width(500)
        .content_height(600)
        .build();

    // Create content
    let toolbar_view = adw::ToolbarView::new();

    // Header bar
    let header = adw::HeaderBar::new();
    header.add_css_class("flat");

    let cancel_button = gtk4::Button::builder()
        .label("Cancel")
        .build();
    header.pack_start(&cancel_button);

    let save_button = gtk4::Button::builder()
        .label("Save")
        .css_classes(vec!["suggested-action"])
        .build();
    header.pack_end(&save_button);

    toolbar_view.add_top_bar(&header);

    // Form content
    let scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    // Name field
    let name_group = adw::PreferencesGroup::builder()
        .title("Server Name")
        .build();
    let name_entry = gtk4::Entry::builder()
        .placeholder_text("e.g., Production Server")
        .build();
    name_group.add(&name_entry);
    content.append(&name_group);

    // Connection info
    let conn_group = adw::PreferencesGroup::builder()
        .title("Connection")
        .build();

    let host_row = adw::EntryRow::builder()
        .title("Host")
        .placeholder_text("e.g., 192.168.1.1 or server.example.com")
        .build();
    conn_group.add(&host_row);

    let port_row = adw::SpinRow::builder()
        .title("Port")
        .adjustment(&gtk4::Adjustment::new(22.0, 1.0, 65535.0, 1.0, 10.0, 0.0))
        .build();
    conn_group.add(&port_row);

    let user_row = adw::EntryRow::builder()
        .title("Username")
        .placeholder_text("e.g., root or admin")
        .build();
    conn_group.add(&user_row);

    content.append(&conn_group);

    // Authentication
    let auth_group = adw::PreferencesGroup::builder()
        .title("Authentication")
        .build();

    // Auth type dropdown
    let auth_model = gtk4::StringList::new(&["Password", "SSH Key", "SSH Agent"]);
    let auth_row = adw::ComboRow::builder()
        .title("Method")
        .model(&auth_model)
        .selected(0u32)
        .build();
    auth_group.add(&auth_row);

    // Password entry (shown when auth type is password)
    let password_entry = adw::PasswordEntryRow::builder()
        .title("Password")
        .show_apply_button(false)
        .build();
    password_entry.set_visible(true);
    auth_group.add(&password_entry);

    // Key file entry (shown when auth type is key)
    let key_row = adw::EntryRow::builder()
        .title("Identity File")
        .text("~/.ssh/id_rsa")
        .build();
    let key_button = gtk4::Button::from_icon_name("document-open-symbolic");
    key_button.set_tooltip_text(Some("Browse..."));
    key_button.set_valign(gtk4::Align::Center);
    key_row.add_suffix(&key_button);
    key_row.set_visible(false);
    auth_group.add(&key_row);

    content.append(&auth_group);

    // Group
    let group_group = adw::PreferencesGroup::builder()
        .title("Organization")
        .build();

    let group_model = gtk4::StringList::new(&["Default", "Production", "Development", "Staging"]);
    let group_row = adw::ComboRow::builder()
        .title("Group")
        .model(&group_model)
        .selected(0u32)
        .build();
    group_group.add(&group_row);

    content.append(&group_group);

    scroll.set_child(Some(&content));
    toolbar_view.set_content(Some(&scroll));

    dialog.set_child(Some(&toolbar_view));

    // Auth type change handler
    auth_row.connect_selected_notify(glib::clone!(@weak password_entry as pw, @weak key_row as key => move |row| {
        let selected = row.selected();
        pw.set_visible(selected == 0);
        key.set_visible(selected == 1);
    }));

    // Cancel button
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    // Save button
    let dialog_weak = dialog.downgrade();
    save_button.connect_clicked(glib::clone!(
        @weak name_entry,
        @weak host_row,
        @weak port_row,
        @weak user_row,
        @weak auth_row,
        @weak password_entry,
        @weak key_row,
        @weak group_row => move |_| {
        let name = name_entry.text().to_string();
        let host = host_row.text().to_string();
        let port = port_row.value() as i64;
        let username = user_row.text().to_string();

        if name.is_empty() || host.is_empty() || username.is_empty() {
            // Show error toast
            return;
        }

        let auth_type = match auth_row.selected() {
            0 => AuthType::Password,
            1 => AuthType::Key,
            _ => AuthType::Agent,
        };

        let identity_file = if auth_type == AuthType::Key {
            Some(key_row.text().to_string())
        } else {
            None
        };

        let server = Server {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            host,
            port,
            username,
            auth_type,
            group_id: Some("default".to_string()),
            status: ServerStatus::Disconnected,
            identity_file,
        };

        callback(server);

        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    }));

    dialog.present(parent);
}
