//! Layout Management
//!
//! Defines the main UI layout structure:
//! - Sidebar (left): Groups list
//! - Main area (center): Server list
//! - Detail panel (right): Server details
//! - Status bar (bottom): Status messages

use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};

/// Calculated layout areas
pub struct LayoutAreas {
    pub sidebar: Rect,
    pub server_list: Rect,
    pub detail_panel: Rect,
    pub status_bar: Rect,
}

/// Manages the UI layout
pub struct LayoutManager {
    sidebar_width: u16,
    detail_width: u16,
    status_height: u16,
}

impl LayoutManager {
    pub fn new() -> Self {
        Self {
            sidebar_width: 20,
            detail_width: 30,
            status_height: 1,
        }
    }

    pub fn update(&mut self, area: Rect) {
        // Adjust layout based on terminal size
        let min_width = 80;
        let min_height = 24;

        if area.width < min_width {
            // Compact mode - reduce sidebar and detail widths
            self.sidebar_width = (area.width / 5).max(10);
            self.detail_width = (area.width / 4).max(15);
        } else {
            // Normal mode
            self.sidebar_width = 20;
            self.detail_width = 30;
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

        // Split main area horizontally into sidebar, server list, and detail panel
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.sidebar_width),
                Constraint::Min(0),
                Constraint::Length(self.detail_width),
            ])
            .split(main_area);

        LayoutAreas {
            sidebar: content_layout[0],
            server_list: content_layout[1],
            detail_panel: content_layout[2],
            status_bar,
        }
    }

    pub fn calculate_dialog_area(&self, area: Rect) -> Rect {
        // Center dialog in the middle 60% of the screen
        let width = (area.width * 3) / 5;
        let height = (area.height * 3) / 5;

        Rect {
            x: (area.width - width) / 2,
            y: (area.height - height) / 2,
            width,
            height,
        }
    }
}
