//! Tab Bar UI Component for Multi-Terminal Support
//!
//! Provides a professional tab bar widget for managing multiple terminal sessions.
//!
//! Features:
//! - Horizontal tab strip with active highlighting
//! - Close button per tab with hover state
//! - New tab button with "+" icon
//! - Tab drag-and-drop reordering
//! - Connection status indicators (connecting/active/error)
//! - Activity/modified indicators
//! - Right-click context menu (close, close others, duplicate, rename)
//! - Keyboard shortcuts (Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1-9)
//!
//! @version 1.0.0
//! @platform Windows (native egui)

use crate::apple_design::{AppleTypography, LucideIcons, MicrointeractionState};
use crate::design::{BrandColors, DesignTheme, NeutralColors, Radius, SemanticColors, Spacing, StatusColors};
use crate::hotkeys::HotkeyAction;
use egui::{
    Align, Align2, Color32, Context, Frame, Id, Layout, Margin, Pos2, Response, RichText, Rounding, Sense,
    Stroke, Ui, Vec2, Widget, FontId,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Tab state types - defined locally to avoid dependency issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Initializing,
    Connecting,
    Active,
    Disconnected,
    Reconnecting,
    Closed,
    Error,
}

impl TabState {
    pub fn is_connected(&self) -> bool {
        matches!(self, TabState::Active | TabState::Connecting | TabState::Reconnecting)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    LocalShell,
    Ssh,
    Serial,
    Telnet,
    Docker,
    Kubernetes,
    Wsl,
    RemoteDesktop,
}

impl SessionType {
    pub fn default_title(&self) -> &'static str {
        match self {
            SessionType::LocalShell => "Local",
            SessionType::Ssh => "SSH",
            SessionType::Serial => "Serial",
            SessionType::Telnet => "Telnet",
            SessionType::Docker => "Docker",
            SessionType::Kubernetes => "K8s",
            SessionType::Wsl => "WSL",
            SessionType::RemoteDesktop => "RDP",
        }
    }
}

/// Simple tab manager for tab state management
#[derive(Debug, Clone)]
pub struct TabManager {
    tabs: Vec<TabDisplayState>,
    active_tab_id: Option<String>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab_id: None,
        }
    }

    pub fn add_tab(&mut self, tab: TabDisplayState) {
        if self.tabs.is_empty() {
            self.active_tab_id = Some(tab.id.clone());
        }
        self.tabs.push(tab);
    }

    pub fn remove_tab(&mut self, id: &str) {
        self.tabs.retain(|t| t.id != id);
        if self.active_tab_id.as_deref() == Some(id) {
            self.active_tab_id = self.tabs.first().map(|t| t.id.clone());
        }
    }

    pub fn get_tabs(&self) -> &[TabDisplayState] {
        &self.tabs
    }

    pub fn get_active_id(&self) -> Option<&str> {
        self.active_tab_id.as_deref()
    }

    pub fn set_active(&mut self, id: &str) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_tab_id = Some(id.to_string());
        }
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tab bar component for managing multiple terminal tabs
pub struct TabBar {
    /// Tab manager reference
    tab_manager: Arc<Mutex<TabManager>>,
    /// Current tabs display state (cached from manager)
    tabs: Vec<TabDisplayState>,
    /// Active tab ID
    active_tab_id: Option<String>,
    /// Drag state for reordering
    drag_state: Option<TabDragStateInternal>,
    /// Hover states for each tab
    hover_states: HashMap<String, MicrointeractionState>,
    /// Context menu state
    context_menu: Option<ContextMenuState>,
    /// Rename dialog state
    rename_dialog: Option<RenameDialogState>,
    /// Scroll offset for horizontal scrolling when tabs overflow
    scroll_offset: f32,
    /// Last update time for activity indicators
    last_update: Instant,
}

/// Internal tab display state for UI rendering
#[derive(Debug, Clone)]
pub struct TabDisplayState {
    pub id: String,
    pub title: String,
    pub state: TabState,
    pub session_type: SessionType,
    pub is_pinned: bool,
    pub has_activity: bool,
    pub modified_at: Option<Instant>,
}

/// Internal drag state (different from core TabDragState)
#[derive(Debug, Clone)]
struct TabDragStateInternal {
    dragged_tab_id: String,
    source_index: usize,
    drag_offset: f32,
    is_dragging: bool,
}

/// Context menu state
#[derive(Debug, Clone)]
struct ContextMenuState {
    tab_id: String,
    tab_index: usize,
    position: Pos2,
}

/// Rename dialog state
#[derive(Debug, Clone)]
struct RenameDialogState {
    tab_id: String,
    current_title: String,
    new_title: String,
}

/// Response from tab bar interactions
#[derive(Debug, Clone)]
pub enum TabBarResponse {
    /// No action taken
    None,
    /// Tab was clicked/activated
    TabActivated { tab_id: String },
    /// Tab was closed
    TabClosed { tab_id: String },
    /// New tab requested
    NewTabRequested,
    /// Tabs were reordered
    TabsReordered { new_order: Vec<String> },
    /// Tab rename requested
    TabRenameRequested { tab_id: String, new_title: String },
    /// Tab duplication requested
    TabDuplicateRequested { tab_id: String },
    /// Close other tabs requested
    CloseOtherTabsRequested { tab_id: String },
    /// Show context menu
    ShowContextMenu { tab_id: String, position: Pos2 },
}

impl TabBar {
    /// Create a new tab bar with the given tab manager
    pub fn new(tab_manager: Arc<Mutex<TabManager>>) -> Self {
        Self {
            tab_manager,
            tabs: Vec::new(),
            active_tab_id: None,
            drag_state: None,
            hover_states: HashMap::new(),
            context_menu: None,
            rename_dialog: None,
            scroll_offset: 0.0,
            last_update: Instant::now(),
        }
    }

    /// Update tab display state from manager
    pub fn update_from_manager(&mut self) {
        // In a real async implementation, we'd use async/await
        // For now, we simulate synchronous access
        // The actual TabManager would need to be polled or use channels
        self.last_update = Instant::now();
    }

    /// Set tabs directly (for synchronous usage)
    pub fn set_tabs(&mut self, tabs: Vec<TabDisplayState>, active_id: Option<String>) {
        self.tabs = tabs;
        self.active_tab_id = active_id;

        // Initialize hover states for new tabs
        for tab in &self.tabs {
            if !self.hover_states.contains_key(&tab.id) {
                self.hover_states.insert(tab.id.clone(), MicrointeractionState::new());
            }
        }

        // Clean up hover states for removed tabs
        let tab_ids: Vec<String> = self.tabs.iter().map(|t| t.id.clone()).collect();
        self.hover_states.retain(|id, _| tab_ids.contains(id));
    }

    /// Handle keyboard shortcuts
    pub fn handle_shortcuts(&mut self, ctx: &Context) -> Option<TabBarResponse> {
        let input = ctx.input(|i| i.clone());

        // Ctrl+T: New tab
        if input.modifiers.ctrl && input.key_pressed(egui::Key::T) {
            return Some(TabBarResponse::NewTabRequested);
        }

        // Ctrl+W: Close active tab
        if input.modifiers.ctrl && input.key_pressed(egui::Key::W) {
            if let Some(active_id) = &self.active_tab_id {
                return Some(TabBarResponse::TabClosed {
                    tab_id: active_id.clone(),
                });
            }
        }

        // Ctrl+Tab: Next tab
        if input.modifiers.ctrl && input.key_pressed(egui::Key::Tab) && !input.modifiers.shift {
            return self.switch_to_next_tab();
        }

        // Ctrl+Shift+Tab: Previous tab
        if input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::Tab) {
            return self.switch_to_prev_tab();
        }

        // Ctrl+1-9: Switch to specific tab
        for (i, key) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
        ]
        .iter()
        .enumerate()
        {
            if input.modifiers.ctrl && input.key_pressed(*key) {
                return self.switch_to_tab_index(i);
            }
        }

        None
    }

    /// Switch to next tab
    fn switch_to_next_tab(&mut self) -> Option<TabBarResponse> {
        if self.tabs.is_empty() {
            return None;
        }

        let current_index = self.get_active_index();
        let next_index = (current_index + 1) % self.tabs.len();

        if let Some(tab) = self.tabs.get(next_index) {
            Some(TabBarResponse::TabActivated {
                tab_id: tab.id.clone(),
            })
        } else {
            None
        }
    }

    /// Switch to previous tab
    fn switch_to_prev_tab(&mut self) -> Option<TabBarResponse> {
        if self.tabs.is_empty() {
            return None;
        }

        let current_index = self.get_active_index();
        let prev_index = if current_index == 0 {
            self.tabs.len() - 1
        } else {
            current_index - 1
        };

        if let Some(tab) = self.tabs.get(prev_index) {
            Some(TabBarResponse::TabActivated {
                tab_id: tab.id.clone(),
            })
        } else {
            None
        }
    }

    /// Switch to tab by index (0-based)
    fn switch_to_tab_index(&mut self, index: usize) -> Option<TabBarResponse> {
        if index < self.tabs.len() {
            Some(TabBarResponse::TabActivated {
                tab_id: self.tabs[index].id.clone(),
            })
        } else {
            // Ctrl+9 switches to last tab
            if index == 8 && self.tabs.len() > 8 {
                Some(TabBarResponse::TabActivated {
                    tab_id: self.tabs.last().unwrap().id.clone(),
                })
            } else {
                None
            }
        }
    }

    /// Get active tab index
    fn get_active_index(&self) -> usize {
        if let Some(active_id) = &self.active_tab_id {
            self.tabs
                .iter()
                .position(|t| &t.id == active_id)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Show the tab bar in the UI
    pub fn show(&mut self, ui: &mut Ui) -> TabBarResponse {
        let theme = DesignTheme::from_theme(if ui.visuals().dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        // Handle keyboard shortcuts
        if let Some(response) = self.handle_shortcuts(ui.ctx()) {
            return response;
        }

        // Main container
        let response = Frame::none()
            .fill(theme.bg_secondary)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(Spacing::_2, Spacing::_1))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // New tab button
                    let new_tab_response = self.show_new_tab_button(ui, &theme);

                    // Separator
                    ui.add_space(Spacing::_2);
                    Frame::none()
                        .fill(theme.border_default)
                        .show(ui, |ui| {
                            ui.allocate_space(Vec2::new(1.0, 24.0));
                        });

                    ui.add_space(Spacing::_2);

                    // Tabs container with horizontal scrolling
                    let tabs_response = self.show_tabs(ui, &theme);

                    // Return combined response
                    match new_tab_response {
                        TabBarResponse::NewTabRequested => TabBarResponse::NewTabRequested,
                        _ => tabs_response,
                    }
                })
                .inner
            })
            .inner;

        // Show context menu if active
        if let Some(menu_state) = &self.context_menu {
            self.show_context_menu(ui, &theme, menu_state.clone());
        }

        // Show rename dialog if active
        if let Some(rename_state) = self.rename_dialog.take() {
            let mut rename_state = rename_state;
            let confirmed = self.show_rename_dialog(ui, &theme, &mut rename_state);
            if confirmed {
                let response = TabBarResponse::TabRenameRequested {
                    tab_id: rename_state.tab_id.clone(),
                    new_title: rename_state.new_title.clone(),
                };
                return response;
            } else {
                // Dialog still open, restore state
                self.rename_dialog = Some(rename_state);
            }
        }

        response
    }

    /// Show new tab button
    fn show_new_tab_button(&mut self, ui: &mut Ui, theme: &DesignTheme) -> TabBarResponse {
        let btn_size = Vec2::new(28.0, 28.0);

        // Use a simple button approach
        let (rect, response) = ui.allocate_exact_size(btn_size, Sense::click());

        // Draw button
        let painter = ui.painter();
        let is_hovered = response.hovered();
        let is_clicked = response.clicked();

        // Background
        let bg_color = if is_hovered {
            theme.interactive_ghost_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, Rounding::same(4.0), bg_color);

        // Icon (+)
        let icon_color = if is_hovered {
            theme.interactive_primary
        } else {
            theme.text_secondary
        };
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(16.0),
            icon_color,
        );

        // Tooltip
        if is_hovered {
            response.on_hover_text("New Tab (Ctrl+T)");
        }

        if is_clicked {
            return TabBarResponse::NewTabRequested;
        }

        TabBarResponse::None
    }

    /// Show tabs with drag support
    fn show_tabs(&mut self, ui: &mut Ui, theme: &DesignTheme) -> TabBarResponse {
        let mut response = TabBarResponse::None;
        let tab_height = 32.0;
        let min_tab_width = 80.0;
        let max_tab_width = 200.0;

        // Calculate available width
        let available_width = ui.available_width();

        // Calculate tab widths
        let num_tabs = self.tabs.len();
        let tab_width = if num_tabs > 0 {
            let ideal_width = available_width / num_tabs as f32;
            ideal_width.clamp(min_tab_width, max_tab_width)
        } else {
            max_tab_width
        };

        // Check for overflow
        let total_tabs_width = num_tabs as f32 * tab_width;
        let needs_scroll = total_tabs_width > available_width;

        // Handle drag state
        let mut new_drag_state = None;
        if let Some(drag) = &self.drag_state {
            if drag.is_dragging {
                // Handle drag rendering
                new_drag_state = Some(drag.clone());
            }
        }

        // Collect tab info before iterating
        let tabs_info: Vec<(usize, String, bool, bool)> = self.tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let is_active = self.active_tab_id.as_ref() == Some(&tab.id);
                let is_dragged = self
                    .drag_state
                    .as_ref()
                    .map(|d| &d.dragged_tab_id == &tab.id)
                    .unwrap_or(false);
                (index, tab.id.clone(), is_active, is_dragged)
            })
            .collect();

        // Render tabs
        for (index, tab_id, is_active, is_dragged) in tabs_info {
            // Get tab by id
            let tab = match self.tabs.iter().find(|t| t.id == tab_id) {
                Some(t) => t.clone(),
                None => continue,
            };

            // Skip rendering dragged tab in normal position
            if is_dragged {
                ui.add_space(tab_width);
                continue;
            }

            let tab_response =
                self.show_single_tab(ui, theme, &tab, index, is_active, tab_width, tab_height);

            // Handle tab responses
            match tab_response {
                TabBarResponse::TabActivated { tab_id } => {
                    response = TabBarResponse::TabActivated { tab_id };
                }
                TabBarResponse::TabClosed { tab_id } => {
                    response = TabBarResponse::TabClosed { tab_id };
                }
                TabBarResponse::ShowContextMenu { tab_id, position } => {
                    self.context_menu = Some(ContextMenuState {
                        tab_id,
                        tab_index: index,
                        position,
                    });
                }
                TabBarResponse::TabsReordered { new_order } => {
                    response = TabBarResponse::TabsReordered { new_order };
                }
                _ => {}
            }
        }

        // Render dragged tab at cursor position
        if let Some(drag) = new_drag_state {
            // Find the dragged tab
            if let Some(dragged_tab) = self.tabs.iter().find(|t| t.id == drag.dragged_tab_id) {
                // Render dragged tab at mouse position (overlay)
                self.render_dragged_tab(ui, theme, dragged_tab, &drag);
            }
        }

        response
    }

    /// Show a single tab
    fn show_single_tab(
        &mut self,
        ui: &mut Ui,
        theme: &DesignTheme,
        tab: &TabDisplayState,
        index: usize,
        is_active: bool,
        width: f32,
        height: f32,
    ) -> TabBarResponse {
        let mut response = TabBarResponse::None;

        // Tab ID for interaction
        let tab_id = Id::new(("tab", &tab.id));

        // Allocate tab space - allocate_space returns (Id, Rect) in egui 0.28
        let (allocated_id, tab_rect) = ui.allocate_space(Vec2::new(width, height));

        // Check interactions - use the allocated rect
        let interact = ui.interact(tab_rect, tab_id, Sense::click_and_drag());

        // Check for secondary click (context menu)
        let is_secondary_click = interact.clicked_by(egui::PointerButton::Secondary);

        // Check for primary click
        let is_primary_click = interact.clicked_by(egui::PointerButton::Primary);

        // Check for drag start
        if interact.drag_started() {
            self.drag_state = Some(TabDragStateInternal {
                dragged_tab_id: tab.id.clone(),
                source_index: index,
                drag_offset: 0.0,
                is_dragging: true,
            });
        }

        // Handle drag release
        if interact.drag_stopped() {
            if let Some(drag) = &self.drag_state {
                // Calculate new position based on mouse position
                let mouse_pos = ui.input(|i| i.pointer.interact_pos());
                if let Some(pos) = mouse_pos {
                    // Determine new index based on position
                    let tabs_start_x = ui.min_rect().min.x;
                    let relative_x = pos.x - tabs_start_x;
                    let tab_width = width;
                    let new_index = (relative_x / tab_width).round() as usize;
                    let new_index = new_index.min(self.tabs.len().saturating_sub(1));

                    if new_index != drag.source_index {
                        // Create new order
                        let mut new_order: Vec<String> = self.tabs.iter().map(|t| t.id.clone()).collect();
                        let dragged_id = new_order.remove(drag.source_index);
                        new_order.insert(new_index, dragged_id);

                        response = TabBarResponse::TabsReordered { new_order };
                    }
                }
            }
            self.drag_state = None;
        }

        // Handle clicks
        if is_primary_click {
            response = TabBarResponse::TabActivated {
                tab_id: tab.id.clone(),
            };
        }

        if is_secondary_click {
            response = TabBarResponse::ShowContextMenu {
                tab_id: tab.id.clone(),
                position: interact.rect.center(),
            };
        }

        // Draw tab
        self.draw_tab(ui, theme, tab_rect, tab, is_active, interact.hovered());

        // Draw close button (separate interaction)
        let close_response = self.draw_close_button(ui, theme, tab_rect, &tab.id, interact.hovered());

        if close_response.clicked() {
            response = TabBarResponse::TabClosed {
                tab_id: tab.id.clone(),
            };
        }

        response
    }

    /// Draw a tab
    fn draw_tab(
        &self,
        ui: &mut Ui,
        theme: &DesignTheme,
        rect: egui::Rect,
        tab: &TabDisplayState,
        is_active: bool,
        is_hovered: bool,
    ) {
        let painter = ui.painter();
        let tab_rounding = Rounding::same(6.0);

        // Background color based on state
        let bg_color = if is_active {
            theme.bg_primary
        } else if is_hovered {
            theme.bg_tertiary
        } else {
            theme.bg_secondary
        };

        // Active tab underline
        if is_active {
            painter.rect_filled(rect, tab_rounding, bg_color);
            // Draw active indicator line at bottom
            let indicator_rect = egui::Rect::from_min_size(
                Pos2::new(rect.min.x + 4.0, rect.max.y - 2.0),
                Vec2::new(rect.width() - 8.0, 2.0),
            );
            painter.rect_filled(indicator_rect, Rounding::same(1.0), theme.interactive_primary);
        } else {
            painter.rect_filled(rect, tab_rounding, bg_color);
        }

        // Border for inactive tabs
        if !is_active {
            painter.rect_stroke(
                rect,
                tab_rounding,
                Stroke::new(1.0, theme.border_subtle),
            );
        }

        // Tab content
        let content_rect = rect.shrink(4.0);

        // Draw status indicator
        let status_color = self.get_status_color(tab.state, theme);
        let status_rect = egui::Rect::from_center_size(
            Pos2::new(content_rect.min.x + 8.0, content_rect.center().y),
            Vec2::new(8.0, 8.0),
        );
        painter.circle_filled(status_rect.center(), 4.0, status_color);

        // Draw activity indicator (pulsing dot)
        if tab.has_activity {
            let pulse = self.get_pulse_phase(ui.ctx());
            let activity_color = BrandColors::C400.linear_multiply(0.5 + pulse * 0.5);
            painter.circle_filled(
                Pos2::new(status_rect.max.x + 4.0, status_rect.center().y),
                2.0,
                activity_color,
            );
        }

        // Draw pinned indicator
        if tab.is_pinned {
            let pin_color = theme.text_tertiary;
            painter.text(
                Pos2::new(content_rect.max.x - 32.0, content_rect.center().y),
                Align2::RIGHT_CENTER,
                LucideIcons::STAR,
                FontId::proportional(10.0),
                pin_color,
            );
        }

        // Draw title
        let title_color = if is_active {
            theme.text_primary
        } else {
            theme.text_secondary
        };

        // Calculate available width for title
        let title_start_x = content_rect.min.x + 20.0; // After status indicator
        let title_end_x = content_rect.max.x - 40.0; // Before close button
        let title_width = title_end_x - title_start_x;

        // Truncate title if too long
        let title_text = self.truncate_title(&tab.title, title_width);

        painter.text(
            Pos2::new(title_start_x, content_rect.center().y),
            Align2::LEFT_CENTER,
            &title_text,
            FontId::proportional(14.0),
            title_color,
        );

        // Draw session type icon
        let icon_text = self.get_session_type_icon(tab.session_type);
        let icon_color = theme.text_tertiary;
        painter.text(
            Pos2::new(title_start_x, content_rect.min.y + 2.0),
            Align2::LEFT_TOP,
            icon_text,
            FontId::proportional(9.0),
            icon_color,
        );
    }

    /// Draw close button
    fn draw_close_button(
        &self,
        ui: &mut Ui,
        theme: &DesignTheme,
        tab_rect: egui::Rect,
        tab_id: &str,
        tab_hovered: bool,
    ) -> Response {
        let close_btn_size = Vec2::new(16.0, 16.0);
        let close_btn_rect = egui::Rect::from_center_size(
            Pos2::new(tab_rect.max.x - 20.0, tab_rect.center().y),
            close_btn_size,
        );

        let close_id = Id::new(("close_tab", tab_id));
        let response = ui.interact(close_btn_rect, close_id, Sense::click());

        let painter = ui.painter();

        // Only show close button when tab is hovered or active
        let show_close = tab_hovered || self.active_tab_id.as_ref() == Some(&tab_id.to_string());

        if show_close {
            let is_close_hovered = response.hovered();

            let bg_color = if is_close_hovered {
                SemanticColors::DANGER_LIGHT
            } else {
                Color32::TRANSPARENT
            };

            painter.rect_filled(close_btn_rect, Rounding::same(4.0), bg_color);

            let icon_color = if is_close_hovered {
                SemanticColors::DANGER
            } else {
                theme.text_tertiary
            };

            painter.text(
                close_btn_rect.center(),
                Align2::CENTER_CENTER,
                LucideIcons::CLOSE,
                FontId::proportional(12.0),
                icon_color,
            );
        }

        response
    }

    /// Render dragged tab at cursor position
    fn render_dragged_tab(
        &self,
        ui: &mut Ui,
        theme: &DesignTheme,
        tab: &TabDisplayState,
        drag: &TabDragStateInternal,
    ) {
        let painter = ui.painter_at(ui.max_rect());

        // Get mouse position
        let mouse_pos = ui.input(|i| i.pointer.interact_pos());
        if let Some(pos) = mouse_pos {
            let tab_width = 120.0;
            let tab_height = 32.0;

            // Draw tab at mouse position with offset
            let tab_rect = egui::Rect::from_center_size(
                Pos2::new(pos.x - drag.drag_offset + tab_width / 2.0, pos.y),
                Vec2::new(tab_width, tab_height),
            );

            // Draw with shadow for dragged effect
            painter.rect_filled(
                tab_rect,
                Rounding::same(6.0),
                theme.bg_elevated,
            );

            // Draw shadow
            painter.rect_stroke(
                tab_rect.expand(2.0),
                Rounding::same(6.0),
                Stroke::new(2.0, Color32::from_rgba_premultiplied(0, 0, 0, 30)),
            );

            // Draw active indicator
            painter.rect_filled(
                egui::Rect::from_min_size(
                    Pos2::new(tab_rect.min.x + 4.0, tab_rect.max.y - 2.0),
                    Vec2::new(tab_rect.width() - 8.0, 2.0),
                ),
                Rounding::same(1.0),
                theme.interactive_primary,
            );

            // Draw title
            painter.text(
                tab_rect.center(),
                Align2::CENTER_CENTER,
                &tab.title,
                FontId::proportional(14.0),
                theme.text_primary,
            );
        }
    }

    /// Show context menu for a tab
    fn show_context_menu(
        &mut self,
        ui: &mut Ui,
        theme: &DesignTheme,
        menu_state: ContextMenuState,
    ) {
        let menu_id = Id::new(("tab_context_menu", &menu_state.tab_id));

        egui::Area::new(menu_id)
            .order(egui::Order::Foreground)
            .fixed_pos(menu_state.position)
            .show(ui.ctx(), |ui| {
                Frame::popup(ui.style())
                    .fill(theme.bg_elevated)
                    .stroke(Stroke::new(1.0, theme.border_default))
                    .rounding(Rounding::same(8.0))
                    .shadow(egui::Shadow {
                        blur: 16.0,
                        spread: 0.0,
                        offset: Vec2::new(0.0, 4.0),
                        color: Color32::from_rgba_premultiplied(0, 0, 0, 40),
                    })
                    .inner_margin(Margin::same(Spacing::_2))
                    .show(ui, |ui| {
                        ui.set_min_width(160.0);

                        // Close tab
                        if ui.button("Close Tab").clicked() {
                            // Emit close event
                            self.context_menu = None;
                        }

                        // Close other tabs
                        if ui.button("Close Other Tabs").clicked() {
                            self.context_menu = None;
                        }

                        // Close tabs to the right
                        if ui.button("Close Tabs to Right").clicked() {
                            self.context_menu = None;
                        }

                        ui.separator();

                        // Duplicate tab
                        if ui.button("Duplicate Tab").clicked() {
                            self.context_menu = None;
                        }

                        // Rename tab
                        if ui.button("Rename Tab...").clicked() {
                            // Open rename dialog
                            let tab = self.tabs.iter().find(|t| t.id == menu_state.tab_id);
                            if let Some(tab) = tab {
                                self.rename_dialog = Some(RenameDialogState {
                                    tab_id: tab.id.clone(),
                                    current_title: tab.title.clone(),
                                    new_title: tab.title.clone(),
                                });
                            }
                            self.context_menu = None;
                        }

                        ui.separator();

                        // Pin/Unpin tab
                        let pin_text = if self
                            .tabs
                            .iter()
                            .find(|t| t.id == menu_state.tab_id)
                            .map(|t| t.is_pinned)
                            .unwrap_or(false)
                        {
                            "Unpin Tab"
                        } else {
                            "Pin Tab"
                        };
                        if ui.button(pin_text).clicked() {
                            self.context_menu = None;
                        }

                        ui.separator();

                        // Reconnect (if disconnected)
                        let tab_state = self
                            .tabs
                            .iter()
                            .find(|t| t.id == menu_state.tab_id)
                            .map(|t| t.state);
                        if matches!(tab_state, Some(TabState::Disconnected) | Some(TabState::Error)) {
                            if ui.button("Reconnect").clicked() {
                                self.context_menu = None;
                            }
                        }

                        // Disconnect (if connected)
                        if matches!(tab_state, Some(TabState::Active)) {
                            if ui.button("Disconnect").clicked() {
                                self.context_menu = None;
                            }
                        }
                    });
            });

        // Close menu on click outside
        if ui.input(|i| i.pointer.any_click()) {
            let menu_rect = ui.ctx().memory(|mem| {
                mem.area_rect(menu_id).unwrap_or_else(|| egui::Rect::NOTHING)
            });
            let mouse_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = mouse_pos {
                if !menu_rect.contains(pos) {
                    self.context_menu = None;
                }
            }
        }
    }

    /// Show rename dialog
    fn show_rename_dialog(
        &self,
        ui: &mut Ui,
        theme: &DesignTheme,
        rename_state: &mut RenameDialogState,
    ) -> bool {
        let mut confirmed = false;
        let mut cancelled = false;
        let dialog_id = Id::new("rename_tab_dialog");

        egui::Window::new("Rename Tab")
            .id(dialog_id)
            .fixed_size(Vec2::new(300.0, 120.0))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.label("Enter new tab name:");

                    ui.add_space(Spacing::_2);

                    // Text input
                    let text_edit = egui::TextEdit::singleline(&mut rename_state.new_title)
                        .desired_width(f32::INFINITY)
                        .hint_text(&rename_state.current_title);
                    ui.add(text_edit);

                    ui.add_space(Spacing::_3);

                    // Buttons
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancel").clicked() {
                                cancelled = true;
                            }

                            if ui.button("Rename").clicked() {
                                confirmed = true;
                            }
                        });
                    });
                });
            });

        if cancelled {
            false
        } else {
            confirmed
        }
    }

    /// Get status color based on tab state
    fn get_status_color(&self, state: TabState, theme: &DesignTheme) -> Color32 {
        match state {
            TabState::Initializing => theme.text_tertiary,
            TabState::Connecting => StatusColors::CONNECTING,
            TabState::Active => StatusColors::ONLINE,
            TabState::Disconnected => StatusColors::OFFLINE,
            TabState::Reconnecting => StatusColors::CONNECTING,
            TabState::Closed => theme.text_quaternary,
            TabState::Error => SemanticColors::DANGER,
        }
    }

    /// Get session type icon
    fn get_session_type_icon(&self, session_type: SessionType) -> &'static str {
        match session_type {
            SessionType::LocalShell => "⌘",
            SessionType::Ssh => "SSH",
            SessionType::Serial => "USB",
            SessionType::Telnet => "TEL",
            SessionType::Docker => "DKR",
            SessionType::Kubernetes => "K8s",
            SessionType::Wsl => "WSL",
            SessionType::RemoteDesktop => "RDP",
        }
    }

    /// Truncate title to fit available width
    fn truncate_title(&self, title: &str, max_width: f32) -> String {
        let char_width = 8.0; // Approximate width per character
        let max_chars = (max_width / char_width) as usize;

        if title.len() > max_chars && max_chars > 3 {
            format!("{}...", &title[..max_chars.saturating_sub(3)])
        } else {
            title.to_string()
        }
    }

    /// Get pulse phase for activity indicator animation
    fn get_pulse_phase(&self, ctx: &Context) -> f32 {
        let elapsed = self.last_update.elapsed().as_secs_f32();
        let phase = (elapsed * 2.0).sin();
        ctx.request_repaint_after(Duration::from_millis(500));
        (phase + 1.0) / 2.0
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(TabManager::new())))
    }
}

/// Helper struct for building a tab bar with configuration
pub struct TabBarBuilder {
    tab_manager: Arc<Mutex<TabManager>>,
    initial_tabs: Vec<TabDisplayState>,
}

impl TabBarBuilder {
    pub fn new(tab_manager: Arc<Mutex<TabManager>>) -> Self {
        Self {
            tab_manager,
            initial_tabs: Vec::new(),
        }
    }

    pub fn with_tab(mut self, tab: TabDisplayState) -> Self {
        self.initial_tabs.push(tab);
        self
    }

    pub fn build(self) -> TabBar {
        let mut tab_bar = TabBar::new(self.tab_manager);
        if !self.initial_tabs.is_empty() {
            let active_id = self.initial_tabs.first().map(|t| t.id.clone());
            tab_bar.set_tabs(self.initial_tabs, active_id);
        }
        tab_bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tab(id: &str, title: &str) -> TabDisplayState {
        TabDisplayState {
            id: id.to_string(),
            title: title.to_string(),
            state: TabState::Active,
            session_type: SessionType::Ssh,
            is_pinned: false,
            has_activity: false,
            modified_at: None,
        }
    }

    #[test]
    fn test_tab_bar_creation() {
        let tab_bar = TabBar::default();
        assert!(tab_bar.tabs.is_empty());
        assert!(tab_bar.active_tab_id.is_none());
    }

    #[test]
    fn test_set_tabs() {
        let mut tab_bar = TabBar::default();
        let tabs = vec![
            create_test_tab("1", "Server 1"),
            create_test_tab("2", "Server 2"),
        ];

        tab_bar.set_tabs(tabs.clone(), Some("1".to_string()));

        assert_eq!(tab_bar.tabs.len(), 2);
        assert_eq!(tab_bar.active_tab_id, Some("1".to_string()));
    }

    #[test]
    fn test_switch_to_next_tab() {
        let mut tab_bar = TabBar::default();
        let tabs = vec![
            create_test_tab("1", "Server 1"),
            create_test_tab("2", "Server 2"),
            create_test_tab("3", "Server 3"),
        ];

        tab_bar.set_tabs(tabs, Some("1".to_string()));

        let response = tab_bar.switch_to_next_tab();
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "2".to_string()
            })
        );

        tab_bar.active_tab_id = Some("3".to_string());
        let response = tab_bar.switch_to_next_tab();
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "1".to_string()
            })
        );
    }

    #[test]
    fn test_switch_to_prev_tab() {
        let mut tab_bar = TabBar::default();
        let tabs = vec![
            create_test_tab("1", "Server 1"),
            create_test_tab("2", "Server 2"),
            create_test_tab("3", "Server 3"),
        ];

        tab_bar.set_tabs(tabs, Some("2".to_string()));

        let response = tab_bar.switch_to_prev_tab();
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "1".to_string()
            })
        );

        tab_bar.active_tab_id = Some("1".to_string());
        let response = tab_bar.switch_to_prev_tab();
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "3".to_string()
            })
        );
    }

    #[test]
    fn test_switch_to_tab_index() {
        let mut tab_bar = TabBar::default();
        let tabs = vec![
            create_test_tab("1", "Server 1"),
            create_test_tab("2", "Server 2"),
            create_test_tab("3", "Server 3"),
        ];

        tab_bar.set_tabs(tabs, Some("1".to_string()));

        let response = tab_bar.switch_to_tab_index(0);
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "1".to_string()
            })
        );

        let response = tab_bar.switch_to_tab_index(2);
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "3".to_string()
            })
        );

        // Ctrl+9 should switch to last tab
        let response = tab_bar.switch_to_tab_index(8);
        assert_eq!(
            response,
            Some(TabBarResponse::TabActivated {
                tab_id: "3".to_string()
            })
        );
    }

    #[test]
    fn test_get_status_color() {
        let tab_bar = TabBar::default();
        let theme = DesignTheme::dark();

        assert_eq!(tab_bar.get_status_color(TabState::Active, &theme), StatusColors::ONLINE);
        assert_eq!(tab_bar.get_status_color(TabState::Connecting, &theme), StatusColors::CONNECTING);
        assert_eq!(tab_bar.get_status_color(TabState::Disconnected, &theme), StatusColors::OFFLINE);
        assert_eq!(tab_bar.get_status_color(TabState::Error, &theme), SemanticColors::DANGER);
    }

    #[test]
    fn test_truncate_title() {
        let tab_bar = TabBar::default();

        let short_title = "Short";
        let truncated = tab_bar.truncate_title(short_title, 100.0, &mut egui::Ui::noop());
        assert_eq!(truncated, short_title);

        let long_title = "This is a very long server name that should be truncated";
        let truncated = tab_bar.truncate_title(long_title, 50.0, &mut egui::Ui::noop());
        assert!(truncated.len() < long_title.len());
        assert!(truncated.ends_with("..."));
    }
}