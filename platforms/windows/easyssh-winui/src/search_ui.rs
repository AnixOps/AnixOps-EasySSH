//! Enhanced Search UI Module
//!
//! Features:
//! - Advanced filtering with multiple criteria
//! - Search history and suggestions
//! - Fuzzy matching with pinyin support
//! - Quick actions for search results
//! - Keyboard navigation
//!
//! Feature flag: search-enhanced

#![allow(dead_code)]

use eframe::egui;
use std::collections::{HashMap, HashSet};

use crate::search::{
    ConnectionStatusFilter, FilterCriteria, GlobalSearchEngine, QuickAction, SearchResult,
    SearchResultType,
};
use crate::viewmodels::ServerViewModel;

/// Enhanced search UI component
pub struct EnhancedSearchUi {
    /// Current search query
    query: String,
    /// Search results
    results: Vec<SearchResult>,
    /// Selected result index
    selected_index: Option<usize>,
    /// Show filter panel
    show_filters: bool,
    /// Filter criteria
    filter: FilterCriteria,
    /// Search history suggestions
    history_suggestions: Vec<String>,
    /// Show search history
    show_history: bool,
    /// Recent searches
    recent_searches: Vec<String>,
    /// Maximum history items
    max_history: usize,
    /// Window dimensions
    window_width: f32,
    window_height: f32,
    /// Callback when result is selected
    on_select: Option<Box<dyn Fn(&SearchResult) + Send + Sync>>,
    /// Callback when search is cancelled
    on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Default for EnhancedSearchUi {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            show_filters: false,
            filter: FilterCriteria::default(),
            history_suggestions: Vec::new(),
            show_history: false,
            recent_searches: Vec::new(),
            max_history: 20,
            window_width: 800.0,
            window_height: 600.0,
            on_select: None,
            on_cancel: None,
        }
    }
}

impl EnhancedSearchUi {
    /// Create new enhanced search UI
    pub fn new() -> Self {
        Self::default()
    }

    /// Set window dimensions
    pub fn with_dimensions(mut self, width: f32, height: f32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    /// Set selection callback
    pub fn on_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(&SearchResult) + Send + Sync + 'static,
    {
        self.on_select = Some(Box::new(callback));
        self
    }

    /// Set cancel callback
    pub fn on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    /// Reset search state
    pub fn reset(&mut self) {
        self.query.clear();
        self.results.clear();
        self.selected_index = None;
        self.show_filters = false;
        self.filter = FilterCriteria::default();
        self.show_history = false;
    }

    /// Add to recent searches
    pub fn add_recent_search(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }
        // Remove duplicate if exists
        self.recent_searches.retain(|s| s != query);
        // Add to front
        self.recent_searches.insert(0, query.to_string());
        // Trim to max
        if self.recent_searches.len() > self.max_history {
            self.recent_searches.pop();
        }
    }

    /// Get search suggestions based on partial input
    pub fn get_suggestions(&self, partial: &str) -> Vec<String> {
        if partial.is_empty() {
            return self.recent_searches.clone();
        }
        self.recent_searches
            .iter()
            .filter(|s| s.to_lowercase().contains(&partial.to_lowercase()))
            .cloned()
            .take(5)
            .collect()
    }

    /// Render the enhanced search dialog
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        search_engine: &mut GlobalSearchEngine,
        servers: &[ServerViewModel],
        favorites: &HashSet<String>,
        command_history: &[String],
        tags: &HashMap<String, Vec<String>>,
        active_sessions: &[String],
    ) -> bool {
        let mut should_close = false;
        let mut should_execute = false;
        let mut selected_result: Option<SearchResult> = None;

        let screen_rect = ctx.screen_rect();
        let window_pos = egui::Pos2::new(
            (screen_rect.width() - self.window_width) / 2.0,
            (screen_rect.height() - self.window_height) / 3.0,
        );

        // Update search results when query changes
        let need_update = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Text(_))));
        if need_update || self.results.is_empty() {
            self.update_results(
                search_engine,
                servers,
                favorites,
                command_history,
                tags,
                active_sessions,
            );
        }

        // Handle keyboard navigation
        self.handle_keyboard(ctx, &mut should_close, &mut should_execute, &mut selected_result);

        // Dark overlay
        ctx.layer_painter(
            egui::LayerId::new(egui::Order::Background, "search_overlay".into()),
        )
        .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(180));

        egui::Window::new("🔍 Enhanced Search")
            .fixed_pos(window_pos)
            .fixed_size([self.window_width, self.window_height])
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(35, 38, 46),
                rounding: egui::Rounding::same(12.0),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 156, 255)),
                shadow: egui::epaint::Shadow {
                    blur: 24.0,
                    spread: 0.0,
                    offset: egui::Vec2::new(0.0, 8.0),
                    color: egui::Color32::from_black_alpha(100),
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Search input header
                    self.render_search_header(ui, &mut should_close);

                    ui.add_space(8.0);

                    // Filter panel
                    if self.show_filters {
                        self.render_filter_panel(ui);
                        ui.add_space(8.0);
                    }

                    // History suggestions (when query is empty)
                    if self.query.is_empty() && !self.recent_searches.is_empty() && self.show_history
                    {
                        self.render_history_suggestions(ui, &mut should_execute, &mut selected_result);
                        ui.add_space(8.0);
                    }

                    ui.separator();

                    // Results count and quick filters
                    self.render_results_header(ui, search_engine);

                    ui.add_space(4.0);

                    // Search results
                    self.render_results_list(
                        ui,
                        &mut should_execute,
                        &mut should_close,
                        &mut selected_result,
                    );

                    ui.separator();

                    // Footer with keyboard shortcuts
                    self.render_footer(ui, &mut should_close);
                });
            });

        // Execute selected result
        if should_execute {
            if let Some(result) = selected_result {
                self.add_recent_search(&self.query.clone());
                if let Some(ref callback) = self.on_select {
                    callback(&result);
                }
            }
        }

        if should_close {
            if let Some(ref callback) = self.on_cancel {
                callback();
            }
        }

        !should_close
    }

    fn render_search_header(&mut self, ui: &mut egui::Ui, should_close: &mut bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🔍").size(24.0));

            let search_edit = egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search servers, commands, snippets... (Ctrl+Shift+F)")
                .font(egui::TextStyle::Heading)
                .desired_width(ui.available_width() - 100.0);

            let response = ui.add(search_edit);
            if self.selected_index.is_some() {
                response.request_focus();
            }

            // Filter toggle button
            let filter_active = self.show_filters || self.has_active_filters();
            let filter_btn = egui::Button::new("⚙ Filters")
                .fill(if filter_active {
                    egui::Color32::from_rgb(64, 156, 255)
                } else {
                    egui::Color32::from_rgb(50, 55, 65)
                })
                .min_size([80.0, 36.0].into());

            if ui.add(filter_btn).clicked() {
                self.show_filters = !self.show_filters;
            }

            // History button
            if ui
                .add(
                    egui::Button::new("🕐")
                        .fill(if self.show_history {
                            egui::Color32::from_rgb(64, 156, 255)
                        } else {
                            egui::Color32::from_rgb(50, 55, 65)
                        })
                        .min_size([36.0, 36.0].into()),
                )
                .clicked()
            {
                self.show_history = !self.show_history;
            }

            // Close button
            if ui
                .add(
                    egui::Button::new("✕")
                        .fill(egui::Color32::from_rgb(80, 60, 60))
                        .min_size([36.0, 36.0].into()),
                )
                .clicked()
            {
                *should_close = true;
            }
        });

        // Show suggestion chips below search bar
        if !self.query.is_empty() && self.results.len() > 0 {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Quick filters:").small());
                ui.add_space(4.0);

                // Tag filters from results
                let mut all_tags: HashSet<String> = HashSet::new();
                for result in &self.results {
                    if let Some(tags) = result.metadata.get("tags") {
                        for tag in tags.split(", ") {
                            if !tag.is_empty() {
                                all_tags.insert(tag.to_string());
                            }
                        }
                    }
                }

                for tag in all_tags.iter().take(5) {
                    let is_active = self.filter.tags.contains(tag);
                    if ui
                        .add(
                            egui::Button::new(format!("# {}", tag))
                                .small()
                                .fill(if is_active {
                                    egui::Color32::from_rgb(64, 156, 255)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                }),
                        )
                        .clicked()
                    {
                        if is_active {
                            self.filter.tags.retain(|t| t != tag);
                        } else {
                            self.filter.tags.push(tag.clone());
                        }
                    }
                }
            });
        }
    }

    fn render_filter_panel(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(45, 50, 60))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Advanced Filters")
                        .strong()
                        .color(egui::Color32::from_rgb(220, 225, 235))
                        .size(14.0),
                );
                ui.add_space(12.0);

                // Connection status filter
                ui.horizontal(|ui| {
                    ui.label("Connection Status:");
                    ui.add_space(8.0);

                    let statuses = vec![
                        ("All", ConnectionStatusFilter::All),
                        ("Connected", ConnectionStatusFilter::Connected),
                        ("Disconnected", ConnectionStatusFilter::Disconnected),
                    ];

                    for (label, status) in &statuses {
                        let is_selected =
                            self.filter.connection_status.as_ref() == Some(status);
                        if ui
                            .add(
                                egui::Button::new(*label).small().fill(if is_selected {
                                    egui::Color32::from_rgb(64, 156, 255)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                }),
                            )
                            .clicked()
                        {
                            self.filter.connection_status = if is_selected {
                                None
                            } else {
                                Some(status.clone())
                            };
                        }
                    }
                });

                ui.add_space(8.0);

                // Favorites and Recent toggles
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new("★ Favorites Only")
                                .small()
                                .fill(if self.filter.only_favorites {
                                    egui::Color32::from_rgb(255, 207, 80)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                }),
                        )
                        .clicked()
                    {
                        self.filter.only_favorites = !self.filter.only_favorites;
                    }

                    if ui
                        .add(
                            egui::Button::new("🕐 Recent Only")
                                .small()
                                .fill(if self.filter.only_recent {
                                    egui::Color32::from_rgb(100, 220, 150)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                }),
                        )
                        .clicked()
                    {
                        self.filter.only_recent = !self.filter.only_recent;
                    }
                });

                ui.add_space(8.0);

                // OS type filter (placeholder)
                ui.horizontal(|ui| {
                    ui.label("OS Type:");
                    ui.add_space(8.0);
                    let os_types = vec!["All", "Linux", "macOS", "Windows", "BSD"];
                    for os in os_types {
                        let is_selected = self.filter.os_type.as_deref() == Some(os);
                        if ui
                            .add(
                                egui::Button::new(os).small().fill(if is_selected {
                                    egui::Color32::from_rgb(64, 156, 255)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                }),
                            )
                            .clicked()
                        {
                            self.filter.os_type = if is_selected { None } else { Some(os.to_string()) };
                        }
                    }
                });

                ui.add_space(12.0);

                // Clear filters button
                if ui
                    .add(
                        egui::Button::new("Clear All Filters")
                            .fill(egui::Color32::from_rgb(80, 60, 60))
                            .min_size([120.0, 28.0].into()),
                    )
                    .clicked()
                {
                    self.filter = FilterCriteria::default();
                }
            });
    }

    fn render_history_suggestions(
        &mut self,
        ui: &mut egui::Ui,
        should_execute: &mut bool,
        selected_result: &mut Option<SearchResult>,
    ) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(45, 50, 60))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Recent Searches")
                        .strong()
                        .color(egui::Color32::from_rgb(150, 160, 175)),
                );
                ui.add_space(8.0);

                for (idx, search) in self.recent_searches.iter().take(8).enumerate() {
                    let response = ui.add(
                        egui::Button::new(format!("🕐 {}", search))
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size([ui.available_width(), 28.0].into()),
                    );

                    if response.clicked() {
                        self.query = search.clone();
                    }
                    if response.hovered() {
                        self.selected_index = Some(idx);
                    }
                }
            });
    }

    fn render_results_header(&mut self, ui: &mut egui::Ui, search_engine: &GlobalSearchEngine) {
        let total_results = self.results.len();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} results", total_results))
                    .small()
                    .color(egui::Color32::from_rgb(150, 160, 175)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Quick filter buttons
                if ui
                    .add(
                        egui::Button::new("★ Favs")
                            .small()
                            .fill(if self.filter.only_favorites {
                                egui::Color32::from_rgb(64, 156, 255)
                            } else {
                                egui::Color32::from_rgb(50, 55, 65)
                            }),
                    )
                    .clicked()
                {
                    self.filter.only_favorites = !self.filter.only_favorites;
                }

                if ui
                    .add(
                        egui::Button::new("🕐 Recent")
                            .small()
                            .fill(if self.filter.only_recent {
                                egui::Color32::from_rgb(64, 156, 255)
                            } else {
                                egui::Color32::from_rgb(50, 55, 65)
                            }),
                    )
                    .clicked()
                {
                    self.filter.only_recent = !self.filter.only_recent;
                }
            });
        });
    }

    fn render_results_list(
        &mut self,
        ui: &mut egui::Ui,
        should_execute: &mut bool,
        should_close: &mut bool,
        selected_result: &mut Option<SearchResult>,
    ) {
        let available_height = self.window_height - 250.0;

        egui::ScrollArea::vertical()
            .max_height(available_height)
            .show(ui, |ui| {
                if self.results.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            egui::RichText::new("No results found")
                                .size(18.0)
                                .color(egui::Color32::from_rgb(150, 160, 175)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Try a different search term or adjust filters")
                                .small()
                                .color(egui::Color32::from_rgb(120, 130, 145)),
                        );
                    });
                } else {
                    let results: Vec<_> = self.results.clone();
                    for (idx, result) in results.iter().enumerate() {
                        let is_selected = self.selected_index == Some(idx);
                        if self.render_result_item(ui, result, is_selected, idx) {
                            *should_execute = true;
                            *selected_result = Some(result.clone());
                        }
                    }
                }
            });
    }

    fn render_result_item(
        &mut self,
        ui: &mut egui::Ui,
        result: &SearchResult,
        is_selected: bool,
        idx: usize,
    ) -> bool {
        let bg_color = if is_selected {
            egui::Color32::from_rgb(64, 120, 200)
        } else {
            egui::Color32::from_rgb(40, 45, 55)
        };
        let text_color = if is_selected {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(220, 225, 235)
        };
        let subtitle_color = if is_selected {
            egui::Color32::from_rgb(200, 210, 230)
        } else {
            egui::Color32::from_rgb(150, 160, 175)
        };

        let response = egui::Frame::none()
            .fill(bg_color)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Icon
                    ui.label(egui::RichText::new(&result.icon).size(22.0));
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        // Title with action hint
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&result.title)
                                    .color(text_color)
                                    .size(15.0)
                                    .strong(),
                            );

                            let action_hint = match result.action {
                                QuickAction::Connect => "[Enter to Connect]",
                                QuickAction::Execute => "[Enter to Execute]",
                                QuickAction::Edit => "[Enter to Edit]",
                                QuickAction::Delete => "[Ctrl+D to Delete]",
                                QuickAction::FilterByTag => "[Enter to Filter]",
                                QuickAction::FilterByGroup => "[Enter to Filter]",
                                QuickAction::CopyToClipboard => "[Enter to Copy]",
                            };

                            ui.label(
                                egui::RichText::new(action_hint)
                                    .small()
                                    .color(if is_selected {
                                        egui::Color32::from_rgb(180, 200, 255)
                                    } else {
                                        egui::Color32::from_rgb(100, 130, 180)
                                    }),
                            );
                        });

                        // Subtitle
                        ui.label(
                            egui::RichText::new(&result.subtitle)
                                .color(subtitle_color)
                                .size(12.0),
                        );

                        // Metadata badges
                        ui.horizontal_wrapped(|ui| {
                            // Tags
                            if let Some(tags) = result.metadata.get("tags") {
                                if !tags.is_empty() {
                                    for tag in tags.split(", ").take(3) {
                                        ui.label(
                                            egui::RichText::new(format!("# {}", tag))
                                                .small()
                                                .color(if is_selected {
                                                    egui::Color32::from_rgb(200, 220, 255)
                                                } else {
                                                    egui::Color32::from_rgb(64, 156, 255)
                                                }),
                                        );
                                    }
                                }
                            }

                            // Favorite badge
                            if let Some(is_fav) = result.metadata.get("is_favorite") {
                                if is_fav == "true" {
                                    ui.label(
                                        egui::RichText::new("★ Favorite")
                                            .small()
                                            .color(egui::Color32::from_rgb(255, 207, 80)),
                                    );
                                }
                            }

                            // Recent badge
                            if let Some(is_recent) = result.metadata.get("is_recent") {
                                if is_recent == "true" {
                                    ui.label(
                                        egui::RichText::new("🕐 Recent")
                                            .small()
                                            .color(egui::Color32::from_rgb(100, 220, 150)),
                                    );
                                }
                            }

                            // Connected badge
                            if let Some(is_conn) = result.metadata.get("is_connected") {
                                if is_conn == "true" {
                                    ui.label(
                                        egui::RichText::new("● Active")
                                            .small()
                                            .color(egui::Color32::from_rgb(72, 199, 116)),
                                    );
                                }
                            }
                        });
                    });
                });
            })
            .response
            .interact(egui::Sense::click());

        let mut clicked = false;

        if response.clicked() {
            self.selected_index = Some(idx);
            clicked = true;
        }

        if response.hovered() {
            self.selected_index = Some(idx);
        }

        // Right-click context menu
        response.context_menu(|ui| {
            match result.result_type {
                SearchResultType::Server => {
                    if ui.button("Connect").clicked() {
                        clicked = true;
                        ui.close_menu();
                    }
                    if ui.button("Edit").clicked() {
                        // Handle edit
                        ui.close_menu();
                    }
                    if ui.button("Duplicate").clicked() {
                        // Handle duplicate
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        // Handle delete
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Add to Favorites").clicked() {
                        // Handle favorite
                        ui.close_menu();
                    }
                }
                SearchResultType::Snippet => {
                    if ui.button("Execute").clicked() {
                        clicked = true;
                        ui.close_menu();
                    }
                    if ui.button("Copy to Clipboard").clicked() {
                        // Handle copy
                        ui.close_menu();
                    }
                    if ui.button("Edit Snippet").clicked() {
                        // Handle edit
                        ui.close_menu();
                    }
                }
                _ => {
                    if ui.button("Select").clicked() {
                        clicked = true;
                        ui.close_menu();
                    }
                }
            }
        });

        clicked
    }

    fn render_footer(&mut self, ui: &mut egui::Ui, _should_close: &mut bool) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(
                    "↑↓ Navigate • Enter Execute • Esc Close • Ctrl+D Delete",
                )
                .small()
                .color(egui::Color32::from_rgb(120, 130, 145)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("Enhanced Search v2.0")
                        .small()
                        .color(egui::Color32::from_rgb(100, 110, 130)),
                );
            });
        });
    }

    fn handle_keyboard(
        &mut self,
        ctx: &egui::Context,
        should_close: &mut bool,
        should_execute: &mut bool,
        selected_result: &mut Option<SearchResult>,
    ) {
        let total_results = self.results.len();

        if total_results == 0 {
            return;
        }

        // Navigation
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some((current + 1).min(total_results - 1));
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some(if current == 0 { 0 } else { current - 1 });
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            self.selected_index = Some(0);
        }

        if ctx.input(|i| i.key_pressed(egui::Key::End)) {
            if total_results > 0 {
                self.selected_index = Some(total_results - 1);
            }
        }

        // Execute selected
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(idx) = self.selected_index {
                if let Some(result) = self.results.get(idx).cloned() {
                    *should_execute = true;
                    *selected_result = Some(result);
                }
            }
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *should_close = true;
        }

        // Delete shortcut
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            if let Some(idx) = self.selected_index {
                if let Some(result) = self.results.get(idx).cloned() {
                    if result.result_type == SearchResultType::Server {
                        // Handle delete - would need access to app state
                    }
                }
            }
        }
    }

    fn update_results(
        &mut self,
        search_engine: &GlobalSearchEngine,
        servers: &[ServerViewModel],
        favorites: &HashSet<String>,
        command_history: &[String],
        tags: &HashMap<String, Vec<String>>,
        active_sessions: &[String],
    ) {
        self.results = search_engine.search(
            &self.query,
            servers,
            favorites,
            command_history,
            tags,
            &self.filter,
            active_sessions,
        );

        // Update selection
        if !self.results.is_empty() {
            if self.selected_index.is_none() {
                self.selected_index = Some(0);
            } else {
                let max_idx = self.results.len().saturating_sub(1);
                if self.selected_index.unwrap() > max_idx {
                    self.selected_index = Some(max_idx);
                }
            }
        } else {
            self.selected_index = None;
        }
    }

    fn has_active_filters(&self) -> bool {
        !self.filter.tags.is_empty()
            || self.filter.group_id.is_some()
            || self.filter.connection_status.is_some()
            || self.filter.os_type.is_some()
            || self.filter.only_favorites
            || self.filter.only_recent
    }

    /// Get current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// Get selected result
    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.selected_index.and_then(|idx| self.results.get(idx))
    }
}

/// Search statistics for analytics
#[derive(Debug, Default)]
pub struct SearchStats {
    pub total_queries: u64,
    pub avg_results_per_query: f32,
    pub most_used_filter: String,
    pub searches_by_type: HashMap<String, u64>,
}
