use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;

use crate::models::ServerGroup;

/// Sidebar component with navigation pattern for GNOME HIG
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
        widget.set_width_request(200);

        // Title header
        let title_label = gtk4::Label::new(Some("Groups"));
        title_label.add_css_class("heading");
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_margin_start(16);
        title_label.set_margin_end(16);
        title_label.set_margin_top(16);
        title_label.set_margin_bottom(12);
        widget.append(&title_label);

        // Create navigation list
        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.add_css_class("navigation-sidebar");

        // Add "All Servers" item
        let all_row = create_navigation_row("All Servers", None, "computer-symbolic", 0);
        list_box.append(&all_row);

        // Separator
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        separator.set_margin_start(8);
        separator.set_margin_end(8);
        widget.append(&separator);

        // Scrolled window for groups
        let scrolled = gtk4::ScrolledWindow::builder()
            .child(&list_box)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .propagate_natural_height(true)
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
        for (index, group) in groups.iter().enumerate() {
            let row = create_navigation_row(
                &format!("{} ({})", group.name, group.server_count),
                Some(&group.id),
                "folder-symbolic",
                index as i32 + 1,
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

    pub fn select_all_servers(&self) {
        if let Some(first_row) = self.list_box.row_at_index(0) {
            self.list_box.select_row(Some(&first_row));
        }
    }

    pub fn get_selected_group(&self) -> Option<String> {
        if let Some(row) = self.list_box.selected_row() {
            if let Some(group_id) = row.data::<String>("group-id") {
                return Some(group_id.as_ref().clone());
            }
        }
        None
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

fn create_navigation_row(
    label: &str,
    group_id: Option<&str>,
    icon_name: &str,
    _index: i32,
) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_selectable(true);
    row.set_activatable(true);

    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(10);
    hbox.set_margin_bottom(10);

    // Icon
    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    icon.add_css_class("dim-label");
    hbox.append(&icon);

    // Label
    let label_widget = gtk4::Label::new(Some(label));
    label_widget.set_halign(gtk4::Align::Start);
    label_widget.set_hexpand(true);
    label_widget.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    label_widget.set_max_width_chars(20);
    hbox.append(&label_widget);

    // Count badge (if applicable)
    if label.contains('(') {
        // Extract count for badge
        if let Some(start) = label.rfind('(') {
            if let Some(end) = label.rfind(')') {
                let count = &label[start + 1..end];
                let badge = gtk4::Label::new(Some(count));
                badge.add_css_class("dim-label");
                badge.add_css_class("caption");
                hbox.append(&badge);
            }
        }
    }

    row.set_child(Some(&hbox));

    if let Some(id) = group_id {
        row.set_data("group-id", id.to_string());
    }

    // Accessibility
    row.set_accessible_role(gtk4::AccessibleRole::ListItem);
    row.set_accessible_label(Some(label));

    row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_creation() {
        // Just verify it compiles
        let _sidebar = Sidebar::new();
    }
}

