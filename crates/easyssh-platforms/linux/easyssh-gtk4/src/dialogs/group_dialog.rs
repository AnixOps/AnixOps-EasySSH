use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::models::ServerGroup;

pub fn show_add_group_dialog<F>(parent: &adw::ApplicationWindow, callback: F)
where
    F: FnOnce(ServerGroup) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("Add Group")
        .content_width(400)
        .content_height(250)
        .build();

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

    // Content
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let group = adw::PreferencesGroup::builder()
        .title("Group Name")
        .description("Create a new group to organize your servers")
        .build();

    let entry = adw::EntryRow::builder()
        .placeholder_text("e.g., Production Servers")
        .build();
    group.add(&entry);

    content.append(&group);

    // Info label
    let info_label = gtk4::Label::builder()
        .label("Groups help you organize servers by environment, project, or team.")
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .css_classes(vec!["dim-label", "caption"])
        .build();
    content.append(&info_label);

    toolbar_view.set_content(Some(&content));
    dialog.set_child(Some(&toolbar_view));

    // Cancel button
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    // Save button
    let dialog_weak = dialog.downgrade();
    save_button.connect_clicked(glib::clone!(@weak entry => move |_| {
        let name = entry.text().to_string();

        if name.is_empty() {
            return;
        }

        let group = ServerGroup {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            server_count: 0,
        };

        callback(group);

        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    }));

    // Enter key handler
    entry.connect_entry_activated(glib::clone!(@weak save_button => move |_| {
        save_button.activate();
    }));

    dialog.present(parent);
}

pub fn show_edit_group_dialog<F>(parent: &adw::ApplicationWindow, group: &ServerGroup, callback: F)
where
    F: FnOnce(ServerGroup) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title("Edit Group")
        .content_width(400)
        .content_height(250)
        .build();

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

    // Content
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    let pref_group = adw::PreferencesGroup::builder()
        .title("Group Name")
        .build();

    let entry = adw::EntryRow::builder()
        .text(&group.name)
        .build();
    pref_group.add(&entry);

    content.append(&pref_group);

    toolbar_view.set_content(Some(&content));
    dialog.set_child(Some(&toolbar_view));

    // Cancel button
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    // Save button
    let group_id = group.id.clone();
    let server_count = group.server_count;
    let dialog_weak = dialog.downgrade();
    save_button.connect_clicked(glib::clone!(@weak entry => move |_| {
        let name = entry.text().to_string();

        if name.is_empty() {
            return;
        }

        let updated_group = ServerGroup {
            id: group_id,
            name,
            server_count,
        };

        callback(updated_group);

        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    }));

    dialog.present(parent);
}
