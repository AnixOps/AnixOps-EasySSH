//! Sidebar Module for EasySSH Lite
//!
//! Provides the left panel containing:
//! - Search box for filtering servers with highlighting
//! - Group filter dropdown
//! - Server cards list with improved styling
//!
//! UI Reference: Termius-style cards with hover states and selection indicators

use crate::design::{BrandColors, DesignTheme, NeutralColors, Radius, Shadows, Spacing};
use crate::terminal_launcher::{SshConnection, TerminalPreference};
use crate::viewmodels::{GroupViewModel, ServerViewModel};
use egui::{
    Align, Color32, Frame, Layout, Margin, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2,
    Widget,
};

/// Search box component with highlight support
pub struct SearchBox {
    pub query: String,
    pub placeholder: String,
    pub focused: bool,
    pub highlight_matches: bool,
}

impl Default for SearchBox {
    fn default() -> Self {
        Self {
            query: String::new(),
            placeholder: "Search servers...".to_string(),
            focused: false,
            highlight_matches: true,
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

    /// Focus the search box programmatically
    pub fn focus(&mut self, ctx: &egui::Context) {
        self.focused = true;
        // Request focus in the next frame
        ctx.memory_mut(|mem| {
            // Note: We can't directly focus the text edit here
            // The focused flag will be checked in show() method
        });
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let theme = DesignTheme::from_theme(if ui.visuals().dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        let response = Frame::none()
            .fill(if self.focused {
                theme.bg_tertiary
            } else {
                theme.bg_secondary
            })
            .rounding(Radius::MD)
            .stroke(Stroke::new(
                if self.focused { 2.0 } else { 1.0 },
                if self.focused {
                    theme.focus_color
                } else {
                    theme.border_default
                },
            ))
            .inner_margin(Margin::same(Spacing::_3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Search icon
                    let icon_color = if self.query.is_empty() {
                        theme.text_tertiary
                    } else {
                        theme.interactive_primary
                    };
                    ui.colored_label(icon_color, "🔍");

                    // Input field
                    let text_edit = egui::TextEdit::singleline(&mut self.query)
                        .hint_text(&self.placeholder)
                        .desired_width(f32::INFINITY)
                        .margin(Margin::same(0.0));
                    let response = ui.add(text_edit);
                    self.focused = response.has_focus();

                    // Clear button with animation
                    if !self.query.is_empty() {
                        ui.scope(|ui| {
                            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                            if ui.small_button("✕").clicked() {
                                self.query.clear();
                                ui.memory_mut(|mem| {
                                    mem.request_focus(response.id);
                                });
                            }
                        });
                    }

                    // Keyboard shortcut hint
                    if !self.focused && self.query.is_empty() {
                        ui.colored_label(theme.text_quaternary, RichText::new("Ctrl+K").size(10.0));
                    }

                    response
                })
                .inner
            })
            .inner;

        response
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

    /// Find and highlight matches in text
    pub fn highlight_text(&self, ui: &mut Ui, text: &str, fallback_color: Color32) {
        if self.query.is_empty() || !self.highlight_matches {
            ui.colored_label(fallback_color, text);
            return;
        }

        let query_lower = self.query.to_lowercase();
        let text_lower = text.to_lowercase();

        // Find matches
        let mut last_end = 0;
        let highlight_color = BrandColors::C400;
        let highlight_bg = BrandColors::C400.linear_multiply(0.2);

        ui.horizontal_wrapped(|ui| {
            for (start, part) in text_lower.match_indices(&query_lower) {
                // Text before match
                if start > last_end {
                    let before = &text[last_end..start];
                    ui.colored_label(fallback_color, before);
                }

                // Highlighted match
                let match_text = &text[start..start + part.len()];
                let _ = Frame::none()
                    .fill(highlight_bg)
                    .rounding(Radius::XS)
                    .inner_margin(Margin::symmetric(2.0, 1.0))
                    .show(ui, |ui| {
                        ui.colored_label(highlight_color, RichText::new(match_text).strong())
                    });

                last_end = start + part.len();
            }

            // Remaining text after last match
            if last_end < text.len() {
                let remaining = &text[last_end..];
                ui.colored_label(fallback_color, remaining);
            }
        });
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

/// Server card display data with group color support
#[derive(Debug, Clone)]
pub struct ServerCardData {
    pub server: ServerViewModel,
    pub is_selected: bool,
    pub is_connected: bool,
    pub terminal_pref: TerminalPreference,
    pub group_color: Option<Color32>,
    pub search_query: String,
}

/// Server card component with Termius-inspired styling
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
        let is_connected = self.data.is_connected;

        // Card styling - Termius-inspired
        let theme = DesignTheme::from_theme(if ui.visuals().dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        // Determine card colors based on state
        let bg_color = if is_selected {
            theme.interactive_primary.linear_multiply(0.15)
        } else if ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            theme.interactive_ghost_hover
        } else {
            theme.bg_secondary
        };

        let stroke_color = if is_selected {
            theme.interactive_primary
        } else if is_connected {
            Color32::GREEN.linear_multiply(0.7)
        } else {
            theme.border_subtle
        };

        let stroke_width = if is_selected { 2.0 } else { 1.0 };

        // Card with group color indicator on left edge
        let group_color = self.data.group_color.unwrap_or(theme.border_subtle);

        let response = Frame::none()
            .fill(bg_color)
            .stroke(Stroke::new(stroke_width, stroke_color))
            .rounding(Radius::LG)
            .inner_margin(Margin::symmetric(Spacing::_4, Spacing::_3))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                // Group color indicator bar on left
                let indicator_width = 4.0;
                let indicator_height = 48.0;
                let indicator_rect = egui::Rect::from_min_size(
                    ui.cursor().min - egui::vec2(Spacing::_4 + indicator_width, 0.0),
                    egui::vec2(indicator_width, indicator_height),
                );
                ui.painter().rect_filled(
                    indicator_rect,
                    Rounding::same(2.0),
                    if is_selected {
                        group_color.linear_multiply(1.3)
                    } else {
                        group_color
                    },
                );

                ui.horizontal(|ui| {
                    // Connection status indicator (pulse animation when connected)
                    let (status_color, pulse) = if is_connected {
                        let pulse = (ui.input(|i| i.time) * 2.0).sin() * 0.3 + 0.7;
                        (Color32::GREEN.linear_multiply(pulse), true)
                    } else {
                        (theme.text_quaternary, false)
                    };

                    let status_pos = ui.cursor().min + egui::vec2(8.0, 12.0);
                    ui.painter()
                        .circle_filled(status_pos, 6.0, theme.bg_secondary);
                    ui.painter().circle_filled(status_pos, 4.0, status_color);

                    if pulse {
                        // Outer ring for connected pulse effect
                        let ring_alpha = ((ui.input(|i| i.time) * 2.0).sin() + 1.0) * 0.15;
                        ui.painter().circle_stroke(
                            status_pos,
                            8.0,
                            Stroke::new(2.0, Color32::GREEN.linear_multiply(ring_alpha)),
                        );
                    }

                    ui.add_space(20.0);

                    // Server name with search highlighting
                    if !self.data.search_query.is_empty() {
                        self.highlight_server_name(ui, &server.name, theme.text_primary);
                    } else {
                        ui.label(
                            RichText::new(&server.name)
                                .strong()
                                .size(14.0)
                                .color(theme.text_primary),
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Connect/Disconnect button with hover effect
                        let (button_text, button_color) = if is_connected {
                            ("Disconnect", Color32::from_rgb(239, 68, 68))
                        } else {
                            ("Connect", theme.interactive_primary)
                        };

                        let button_response = ui.add(
                            egui::Button::new(button_text)
                                .fill(if is_connected {
                                    button_color.linear_multiply(0.15)
                                } else {
                                    button_color
                                })
                                .stroke(Stroke::new(1.0, button_color))
                                .rounding(Radius::SM),
                        );

                        if button_response.clicked() {
                            // Connection action will be handled by response
                        }
                    });
                });

                ui.add_space(4.0);

                // Host info with terminal icon
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.colored_label(theme.text_tertiary, "▸");
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{}@{}:{}",
                            server.username, server.host, server.port
                        ))
                        .size(12.0)
                        .monospace()
                        .color(theme.text_secondary),
                    );
                });

                ui.add_space(4.0);

                // Auth type badge and group info
                ui.horizontal(|ui| {
                    ui.add_space(20.0);

                    // Auth type badge with icon
                    let (auth_icon, auth_label, auth_color) = match server.auth_type.as_str() {
                        "agent" => ("🔐", "SSH Agent", Color32::from_rgb(59, 130, 246)),
                        "key" => ("🗝", "Key", Color32::from_rgb(168, 85, 247)),
                        "password" => ("🔑", "Password", Color32::from_rgb(234, 179, 8)),
                        _ => ("●", &server.auth_type, theme.text_tertiary),
                    };

                    Frame::none()
                        .fill(auth_color.linear_multiply(0.1))
                        .rounding(Radius::SM)
                        .inner_margin(Margin::symmetric(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(auth_color.linear_multiply(0.8), auth_icon);
                                ui.colored_label(auth_color, RichText::new(auth_label).size(11.0));
                            });
                        });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Connection indicator
                        if is_connected {
                            ui.colored_label(
                                Color32::GREEN,
                                RichText::new("● Connected").size(10.0).strong(),
                            );
                        }
                    });
                });
            })
            .response;

        let clicked = response.clicked();
        let connect_clicked = clicked; // Simplified - actual connect handled separately

        ServerCardResponse {
            clicked,
            connect_clicked,
            server_id: server.id.clone(),
        }
    }

    fn highlight_server_name(&self, ui: &mut Ui, text: &str, fallback_color: Color32) {
        let query = &self.data.search_query;
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();

        let highlight_color = BrandColors::C400;
        let highlight_bg = BrandColors::C400.linear_multiply(0.2);

        ui.horizontal(|ui| {
            let mut last_end = 0;
            let base_text = RichText::new("").size(14.0).strong();

            for (start, part) in text_lower.match_indices(&query_lower) {
                if start > last_end {
                    let before = &text[last_end..start];
                    ui.colored_label(fallback_color, RichText::new(before).size(14.0).strong());
                }

                let match_text = &text[start..start + part.len()];
                let _ = Frame::none()
                    .fill(highlight_bg)
                    .rounding(Radius::XS)
                    .inner_margin(Margin::symmetric(2.0, 0.0))
                    .show(ui, |ui| {
                        ui.colored_label(
                            highlight_color,
                            RichText::new(match_text).size(14.0).strong(),
                        )
                    });

                last_end = start + part.len();
            }

            if last_end < text.len() {
                let remaining = &text[last_end..];
                ui.colored_label(fallback_color, RichText::new(remaining).size(14.0).strong());
            }
        });
    }
}

/// Server card response
#[derive(Debug, Clone)]
pub struct ServerCardResponse {
    pub clicked: bool,
    pub connect_clicked: bool,
    pub server_id: String,
}

/// Server list component with virtual scrolling
pub struct ServerList {
    pub servers: Vec<ServerCardData>,
    pub search_query: String,
}

impl ServerList {
    pub fn new(servers: Vec<ServerCardData>, search_query: String) -> Self {
        Self {
            servers,
            search_query,
        }
    }

    pub fn show(&self, ui: &mut Ui) -> Vec<ServerCardResponse> {
        let mut responses = Vec::new();

        // Empty state
        if self.servers.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.colored_label(
                    ui.visuals().weak_text_color(),
                    RichText::new("No servers found").size(14.0),
                );
                if !self.search_query.is_empty() {
                    ui.colored_label(
                        ui.visuals().weak_text_color(),
                        RichText::new("Try a different search term").size(12.0),
                    );
                }
            });
            return responses;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = Spacing::_2;

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
    pub group_colors: std::collections::HashMap<String, Color32>,
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
            group_colors: std::collections::HashMap::new(),
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

        // Update group colors with default palette
        let default_colors = [
            Color32::from_rgb(59, 130, 246), // Blue
            Color32::from_rgb(34, 197, 94),  // Green
            Color32::from_rgb(239, 68, 68),  // Red
            Color32::from_rgb(249, 115, 22), // Orange
            Color32::from_rgb(168, 85, 247), // Purple
            Color32::from_rgb(236, 72, 153), // Pink
            Color32::from_rgb(6, 182, 212),  // Cyan
            Color32::from_rgb(234, 179, 8),  // Yellow
        ];

        for (i, group) in self.groups.iter().enumerate() {
            if !self.group_colors.contains_key(&group.id) {
                let color = default_colors[i % default_colors.len()];
                self.group_colors.insert(group.id.clone(), color);
            }
        }
    }

    /// Set custom color for a group
    pub fn set_group_color(&mut self, group_id: String, color: Color32) {
        self.group_colors.insert(group_id, color);
    }

    /// Get color for a group
    pub fn get_group_color(&self, group_id: &str) -> Option<Color32> {
        self.group_colors.get(group_id).copied()
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

            // Server count with search indicator
            let filtered_count = self.get_filtered_servers().len();
            let total_count = self.servers.len();
            let count_text = if filtered_count == total_count {
                format!("{} servers", filtered_count)
            } else {
                format!("{} of {} servers", filtered_count, total_count)
            };
            ui.label(
                RichText::new(count_text)
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
            );

            // Server list with search query and group colors
            let search_query = self.search_box.query.clone();
            let server_cards: Vec<ServerCardData> = self
                .get_filtered_servers()
                .into_iter()
                .map(|server| {
                    let group_color = server
                        .group_id
                        .as_ref()
                        .and_then(|id| self.group_colors.get(id).cloned());

                    ServerCardData {
                        server: server.clone(),
                        is_selected: self.selected_server_id.as_ref() == Some(&server.id),
                        is_connected: self.connected_servers.contains(&server.id),
                        terminal_pref: TerminalPreference::Auto,
                        group_color,
                        search_query: search_query.clone(),
                    }
                })
                .collect();

            let list = ServerList::new(server_cards, search_query);
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
