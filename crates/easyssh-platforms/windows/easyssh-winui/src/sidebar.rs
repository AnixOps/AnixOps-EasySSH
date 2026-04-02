//! Sidebar Module for EasySSH Lite
//!
//! Provides the left panel containing:
//! - Search box for filtering servers
//! - Group filter dropdown
//! - Server cards list

use crate::terminal_launcher::{SshConnection, TerminalPreference};
use crate::viewmodels::{GroupViewModel, ServerViewModel};
use egui::{Align, Color32, Layout, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2, Widget};

/// Search box component
pub struct SearchBox {
    pub query: String,
    pub placeholder: String,
}

impl Default for SearchBox {
    fn default() -> Self {
        Self {
            query: String::new(),
            placeholder: "Search servers...".to_string(),
        }
    }
}

impl SearchBox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(&self.placeholder)
                    .desired_width(f32::INFINITY),
            );

            // Clear button
            if !self.query.is_empty() {
                if ui.button("✕").clicked() {
                    self.query.clear();
                }
            }

            response
        })
        .inner
    }

    pub fn clear(&mut self) {
        self.query.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    pub fn matches(&self, text: &str) -> bool {
        if self.query.is_empty() {
            return true;
        }
        text.to_lowercase().contains(&self.query.to_lowercase())
    }
}

/// Group filter component
pub struct GroupFilter {
    pub selected_group: Option<String>,
    pub all_label: String,
}

impl Default for GroupFilter {
    fn default() -> Self {
        Self {
            selected_group: None,
            all_label: "All Groups".to_string(),
        }
    }
}

impl GroupFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui, groups: &[GroupViewModel]) -> Option<String> {
        let mut changed = None;

        ui.horizontal(|ui| {
            ui.label("Group:");

            egui::ComboBox::from_id_source("group_filter")
                .width(200.0)
                .selected_text(
                    self.selected_group
                        .as_ref()
                        .and_then(|id| groups.iter().find(|g| &g.id == id))
                        .map(|g| g.name.clone())
                        .unwrap_or_else(|| self.all_label.clone()),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.selected_group.is_none(), &self.all_label)
                        .clicked()
                    {
                        self.selected_group = None;
                        changed = None;
                    }

                    ui.separator();

                    for group in groups {
                        let is_selected = self.selected_group.as_ref() == Some(&group.id);
                        if ui.selectable_label(is_selected, &group.name).clicked() {
                            self.selected_group = Some(group.id.clone());
                            changed = Some(group.id.clone());
                        }
                    }
                });

            // Clear filter button
            if self.selected_group.is_some() {
                if ui.small_button("Clear").clicked() {
                    self.selected_group = None;
                    changed = None;
                }
            }
        });

        changed
    }

    pub fn clear(&mut self) {
        self.selected_group = None;
    }

    pub fn matches(&self, server_group_id: Option<&String>) -> bool {
        match &self.selected_group {
            None => true, // No filter selected, matches all
            Some(filter_id) => server_group_id.map(|id| id == filter_id).unwrap_or(false),
        }
    }
}

/// Server card display data
#[derive(Debug, Clone)]
pub struct ServerCardData {
    pub server: ServerViewModel,
    pub is_selected: bool,
    pub is_connected: bool,
    pub terminal_pref: TerminalPreference,
}

/// Server card component
pub struct ServerCard {
    data: ServerCardData,
}

impl ServerCard {
    pub fn new(data: ServerCardData) -> Self {
        Self { data }
    }

    pub fn show(&self, ui: &mut Ui) -> ServerCardResponse {
        let server = &self.data.server;
        let is_selected = self.data.is_selected;

        // Card styling
        let card_color = if is_selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().panel_fill
        };

        let stroke = if is_selected {
            Stroke::new(2.0, ui.visuals().selection.stroke.color)
        } else {
            ui.visuals().widgets.inactive.bg_stroke
        };

        let response = egui::Frame::none()
            .fill(card_color)
            .stroke(stroke)
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Server name and status
                ui.horizontal(|ui| {
                    // Status indicator
                    let status_color = if self.data.is_connected {
                        Color32::GREEN
                    } else {
                        Color32::GRAY
                    };
                    ui.painter()
                        .circle_filled(ui.cursor().min, 6.0, status_color);
                    ui.add_space(8.0);

                    // Server name
                    ui.label(RichText::new(&server.name).strong().size(14.0));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Connect button
                        let connect_text = if self.data.is_connected {
                            "Disconnect"
                        } else {
                            "Connect"
                        };

                        if ui.button(connect_text).clicked() {
                            // Connection action will be handled by response
                        }
                    });
                });

                ui.add_space(4.0);

                // Host info
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}@{}:{}",
                            server.username, server.host, server.port
                        ))
                        .size(12.0)
                        .color(ui.visuals().weak_text_color()),
                    );
                });

                ui.add_space(4.0);

                // Auth type badge
                let auth_label = match server.auth_type.as_str() {
                    "agent" => "SSH Agent",
                    "key" => "Key",
                    "password" => "Password",
                    _ => &server.auth_type,
                };

                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(auth_label)
                            .size(11.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                });
            })
            .response;

        let clicked = response.clicked();
        let connect_clicked = clicked; // Simplified

        ServerCardResponse {
            clicked,
            connect_clicked,
            server_id: server.id.clone(),
        }
    }
}

/// Server card response
#[derive(Debug, Clone)]
pub struct ServerCardResponse {
    pub clicked: bool,
    pub connect_clicked: bool,
    pub server_id: String,
}

/// Server list component
pub struct ServerList {
    pub servers: Vec<ServerCardData>,
}

impl ServerList {
    pub fn new(servers: Vec<ServerCardData>) -> Self {
        Self { servers }
    }

    pub fn show(&self, ui: &mut Ui) -> Vec<ServerCardResponse> {
        let mut responses = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                for server_data in &self.servers {
                    let card = ServerCard::new(server_data.clone());
                    let response = card.show(ui);
                    responses.push(response);
                }
            });

        responses
    }
}

/// Sidebar component containing search, filter, and server list
pub struct Sidebar {
    pub search_box: SearchBox,
    pub group_filter: GroupFilter,
    pub selected_server_id: Option<String>,
    pub servers: Vec<ServerViewModel>,
    pub groups: Vec<GroupViewModel>,
    pub connected_servers: Vec<String>, // IDs of connected servers
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            search_box: SearchBox::new(),
            group_filter: GroupFilter::new(),
            selected_server_id: None,
            servers: Vec::new(),
            groups: Vec::new(),
            connected_servers: Vec::new(),
        }
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_data(
        &mut self,
        servers: Vec<ServerViewModel>,
        groups: Vec<GroupViewModel>,
        connected: Vec<String>,
    ) {
        self.servers = servers;
        self.groups = groups;
        self.connected_servers = connected;
    }

    pub fn show(&mut self, ui: &mut Ui) -> SidebarResponse {
        let mut response = SidebarResponse::default();

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;

            // Header
            ui.heading("Servers");

            ui.separator();

            // Search box
            self.search_box.show(ui);

            // Group filter
            if let Some(group_id) = self.group_filter.show(ui, &self.groups) {
                response.group_changed = Some(group_id);
            }

            ui.separator();

            // Server count
            let filtered_count = self.get_filtered_servers().len();
            ui.label(
                RichText::new(format!("{} servers", filtered_count))
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
            );

            // Server list
            let server_cards: Vec<ServerCardData> = self
                .get_filtered_servers()
                .into_iter()
                .map(|server| ServerCardData {
                    server: server.clone(),
                    is_selected: self.selected_server_id.as_ref() == Some(&server.id),
                    is_connected: self.connected_servers.contains(&server.id),
                    terminal_pref: TerminalPreference::Auto,
                })
                .collect();

            let list = ServerList::new(server_cards);
            let card_responses = list.show(ui);

            for card_response in card_responses {
                if card_response.clicked {
                    self.selected_server_id = Some(card_response.server_id.clone());
                    response.server_selected = Some(card_response.server_id.clone());
                }
                if card_response.connect_clicked {
                    response.connect_clicked = Some(card_response.server_id);
                }
            }
        });

        response
    }

    fn get_filtered_servers(&self) -> Vec<&ServerViewModel> {
        self.servers
            .iter()
            .filter(|server| {
                // Search filter
                let search_match = self.search_box.matches(&server.name)
                    || self.search_box.matches(&server.host)
                    || self.search_box.matches(&server.username);

                // Group filter
                let group_match = self.group_filter.matches(server.group_id.as_ref());

                search_match && group_match
            })
            .collect()
    }

    pub fn select_server(&mut self, server_id: Option<String>) {
        self.selected_server_id = server_id;
    }

    pub fn get_selected_server(&self) -> Option<&ServerViewModel> {
        self.selected_server_id
            .as_ref()
            .and_then(|id| self.servers.iter().find(|s| &s.id == id))
    }
}

/// Sidebar response
#[derive(Debug, Default)]
pub struct SidebarResponse {
    pub server_selected: Option<String>,
    pub connect_clicked: Option<String>,
    pub group_changed: Option<String>,
}

/// Quick actions bar
pub struct QuickActionsBar;

impl QuickActionsBar {
    pub fn show(ui: &mut Ui) -> QuickActionsResponse {
        let mut response = QuickActionsResponse::default();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if ui.button("➕ Add Server").clicked() {
                response.add_server = true;
            }

            if ui.button("📁 Groups").clicked() {
                response.manage_groups = true;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("⚙").clicked() {
                    response.open_settings = true;
                }
            });
        });

        response
    }
}

/// Quick actions response
#[derive(Debug, Default)]
pub struct QuickActionsResponse {
    pub add_server: bool,
    pub manage_groups: bool,
    pub open_settings: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_box_matches() {
        let mut search = SearchBox::new();

        // Empty query matches everything
        assert!(search.matches("anything"));

        search.query = "test".to_string();
        assert!(search.matches("Test Server"));
        assert!(search.matches("my test"));
        assert!(!search.matches("other"));

        // Case insensitive
        search.query = "SERVER".to_string();
        assert!(search.matches("server"));
        assert!(search.matches("My Server"));
    }

    #[test]
    fn test_group_filter_matches() {
        let mut filter = GroupFilter::new();

        // No filter selected matches all
        assert!(filter.matches(Some(&"group1".to_string())));
        assert!(filter.matches(None));

        // Filter selected
        filter.selected_group = Some("group1".to_string());
        assert!(filter.matches(Some(&"group1".to_string())));
        assert!(!filter.matches(Some(&"group2".to_string())));
        assert!(!filter.matches(None));
    }

    #[test]
    fn test_server_card_response() {
        let response = ServerCardResponse {
            clicked: true,
            connect_clicked: false,
            server_id: "srv-123".to_string(),
        };

        assert!(response.clicked);
        assert!(!response.connect_clicked);
        assert_eq!(response.server_id, "srv-123");
    }

    #[test]
    fn test_sidebar_response_default() {
        let response = SidebarResponse::default();
        assert!(response.server_selected.is_none());
        assert!(response.connect_clicked.is_none());
        assert!(response.group_changed.is_none());
    }

    #[test]
    fn test_quick_actions_response() {
        let response = QuickActionsResponse {
            add_server: true,
            manage_groups: false,
            open_settings: true,
        };

        assert!(response.add_server);
        assert!(!response.manage_groups);
        assert!(response.open_settings);
    }
}
