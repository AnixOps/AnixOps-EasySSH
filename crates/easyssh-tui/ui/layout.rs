//! Layout Management
//!
//! Defines the main UI layout structure inspired by ranger/htop:
//! - Sidebar (left): Groups list
//! - Main area (center): Server list with virtual scrolling
//! - Detail panel (right): Server details
//! - Status bar (bottom): Status messages and key hints
//!
//! Layout is responsive and adapts to terminal size.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout mode based on terminal size
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutMode {
    /// Full layout with all panels
    Full,
    /// Compact - reduced sidebar/detail, or hide detail
    Compact,
    /// Minimal - sidebar only, detail in popup
    Minimal,
}

/// Calculated layout areas
pub struct LayoutAreas {
    pub sidebar: Rect,
    pub server_list: Rect,
    pub detail_panel: Rect,
    pub status_bar: Rect,
    pub mode: LayoutMode,
}

/// Manages the UI layout
pub struct LayoutManager {
    sidebar_width: u16,
    detail_width: u16,
    status_height: u16,
    mode: LayoutMode,
    min_width: u16,
    min_height: u16,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            sidebar_width: 20,
            detail_width: 32,
            status_height: 1,
            mode: LayoutMode::Full,
            min_width: 80,
            min_height: 24,
        }
    }

    pub fn update(&mut self, area: Rect) {
        // Determine layout mode based on terminal size
        self.mode = if area.width < 60 {
            LayoutMode::Minimal
        } else if area.width < 100 {
            LayoutMode::Compact
        } else {
            LayoutMode::Full
        };

        // Adjust layout based on terminal size
        match self.mode {
            LayoutMode::Full => {
                self.sidebar_width = (area.width / 5).clamp(15, 30);
                self.detail_width = (area.width / 4).clamp(20, 40);
            }
            LayoutMode::Compact => {
                self.sidebar_width = (area.width / 6).clamp(12, 20);
                self.detail_width = (area.width / 5).clamp(15, 25);
            }
            LayoutMode::Minimal => {
                self.sidebar_width = (area.width / 4).clamp(10, 18);
                self.detail_width = 0; // Hidden, shown in popup
            }
        }
    }

    pub fn calculate_areas(&self, area: Rect) -> LayoutAreas {
        // Split vertically into main content and status bar
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(self.status_height)])
            .split(area);

        let main_area = main_layout[0];
        let status_bar = main_layout[1];

        // Calculate remaining width for server list
        let server_list_width = main_area
            .width
            .saturating_sub(self.sidebar_width + self.detail_width);

        // Split main area horizontally
        let content_layout = if self.mode == LayoutMode::Minimal {
            // Minimal mode: only sidebar and server list
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(self.sidebar_width), Constraint::Min(0)])
                .split(main_area)
        } else {
            // Full/Compact mode: all three panels
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(self.sidebar_width),
                    Constraint::Min(server_list_width),
                    Constraint::Length(self.detail_width),
                ])
                .split(main_area)
        };

        LayoutAreas {
            sidebar: content_layout[0],
            server_list: content_layout[1],
            detail_panel: if self.mode == LayoutMode::Minimal {
                Rect::default() // Not used in minimal mode
            } else {
                content_layout[2]
            },
            status_bar,
            mode: self.mode,
        }
    }

    pub fn calculate_dialog_area(&self, area: Rect) -> Rect {
        // Center dialog in the middle 60-70% of the screen
        let width = (area.width * 3) / 5;
        let height = (area.height * 2) / 3;

        // Ensure minimum size
        let width = width.max(50).min(area.width - 4);
        let height = height.max(15).min(area.height - 4);

        Rect {
            x: (area.width - width) / 2,
            y: (area.height - height) / 2,
            width,
            height,
        }
    }

    pub fn calculate_popup_area(&self, area: Rect, popup_width: u16, popup_height: u16) -> Rect {
        let width = popup_width.min(area.width - 4);
        let height = popup_height.min(area.height - 4);

        Rect {
            x: (area.width - width) / 2,
            y: (area.height - height) / 2,
            width,
            height,
        }
    }

    pub fn get_mode(&self) -> LayoutMode {
        self.mode
    }

    /// Check if detail panel should be shown in popup (minimal mode)
    pub fn detail_in_popup(&self) -> bool {
        self.mode == LayoutMode::Minimal
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}
