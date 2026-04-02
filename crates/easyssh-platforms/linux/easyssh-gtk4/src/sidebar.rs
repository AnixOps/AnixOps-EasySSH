use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;

use crate::models::ServerGroup;

pub struct Sidebar {
    widget: gtk4::Box,
    list_box: gtk4::ListBox,
    groups: RefCell<Vec<ServerGroup>>,
    selected_callback: RefCell<Option<Box<dyn Fn(Option<String>) + 'static>>>,
}

impl Sidebar {
    pub fn new() -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        widget.add_css_class("sidebar");

        // Title
        let title_label = gtk4::Label::new(Some("Groups"));
        title_label.add_css_class("title-4");
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_margin_start(12);
        title_label.set_margin_end(12);
        title_label.set_margin_top(12);
        title_label.set_margin_bottom(6);
        widget.append(&title_label);

        // All servers row
        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.add_css_class("navigation-sidebar");

        // Add "All Servers" item
        let all_row = create_group_row("All Servers", None, "computer-symbolic");
        list_box.append(&all_row);

        // Separator
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        widget.append(&separator);

        // Scrolled window for groups
        let scrolled = gtk4::ScrolledWindow::builder()
            .child(&list_box)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .build();

        widget.append(&scrolled);

        let sidebar = Self {
            widget,
            list_box,
            groups: RefCell::new(Vec::new()),
            selected_callback: RefCell::new(None),
        };

        sidebar.setup_signals();
        sidebar
    }

    fn setup_signals(&self) {
        let callback_cell = self.selected_callback.clone();

        self.list_box.connect_row_selected(move |_, row| {
            if let Some(ref callback) = *callback_cell.borrow() {
                if let Some(row) = row {
                    if let Some(group_id) = row.data::<String>("group-id") {
                        callback(Some(group_id.as_ref().clone()));
                    } else {
                        callback(None); // All servers
                    }
                }
            }
        });
    }

    pub fn set_groups(&self, groups: Vec<ServerGroup>) {
        // Clear existing group rows (keep "All Servers" and separator)
        while let Some(row) = self.list_box.row_at_index(1) {
            self.list_box.remove(&row);
        }

        // Add new groups
        for group in &groups {
            let row = create_group_row(
                &format!("{} ({})", group.name, group.server_count),
                Some(&group.id),
                "folder-symbolic",
            );
            self.list_box.append(&row);
        }

        self.groups.replace(groups);
    }

    pub fn connect_group_selected<F>(&self, callback: F)
    where
        F: Fn(Option<String>) + 'static,
    {
        self.selected_callback.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

impl Clone for Sidebar {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            list_box: self.list_box.clone(),
            groups: self.groups.clone(),
            selected_callback: RefCell::new(None),
        }
    }
}

fn create_group_row(label: &str, group_id: Option<&str>, icon_name: &str) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();

    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    hbox.append(&icon);

    let label_widget = gtk4::Label::new(Some(label));
    label_widget.set_halign(gtk4::Align::Start);
    label_widget.set_hexpand(true);
    hbox.append(&label_widget);

    row.set_child(Some(&hbox));

    if let Some(id) = group_id {
        row.set_data("group-id", id.to_string());
    }

    row
}
