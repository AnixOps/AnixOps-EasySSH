//! UI Module
//!
//! This module provides all UI components:
//! - Layout management
//! - Server list rendering
//! - Sidebar rendering
//! - Detail panel rendering
//! - Dialog rendering

pub mod dialogs;
pub mod layout;
pub mod server_list;
pub mod sidebar;
pub mod detail_panel;

use crate::app::App;
use ratatui::Frame;

pub struct Ui {
    layout: layout::LayoutManager,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            layout: layout::LayoutManager::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, app: &mut App) {
        // Update layout based on terminal size
        self.layout.update(frame.size());

        // Clear background
        frame.render_widget(
            ratatui::widgets::Clear,
            frame.size(),
        );

        // Render main layout areas
        let areas = self.layout.calculate_areas(frame.size());

        // Render sidebar (groups)
        sidebar::render(frame, areas.sidebar, app);

        // Render server list
        server_list::render(frame, areas.server_list, app);

        // Render detail panel
        detail_panel::render(frame, areas.detail_panel, app);

        // Render status bar
        self.render_status_bar(frame, areas.status_bar, app);

        // Render dialog overlay if present
        if let Some(dialog) = &mut app.dialog {
            let dialog_area = self.layout.calculate_dialog_area(frame.size());
            dialog.render(frame, dialog_area);
        }

        // Render search overlay if in search mode
        if app.view_mode == crate::app::ViewMode::Search {
            self.render_search_bar(frame, app);
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let version = env!("CARGO_PKG_VERSION");
        let server_count = app.get_filtered_server_count();
        let total_count = app.get_total_server_count();

        let status_text = if app.view_mode == crate::app::ViewMode::Search {
            format!(
                " EasySSH Lite v{} | {} of {} servers | Search: {}",
                version, server_count, total_count, app.search_query
            )
        } else {
            format!(
                " EasySSH Lite v{} | {} of {} servers | {}",
                version, server_count, total_count, app.status_message
            )
        };

        let status = Paragraph::new(Line::from(vec![
            Span::styled(status_text, Style::default().fg(Color::White).bg(Color::Blue)),
        ]));

        frame.render_widget(status, area);
    }

    fn render_search_bar(&self, frame: &mut Frame, app: &App) {
        use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Clear, Paragraph};

        let area = frame.size();
        let search_height = 3;
        let search_area = Rect {
            x: area.width / 4,
            y: area.height / 2 - search_height / 2,
            width: area.width / 2,
            height: search_height,
        };

        // Clear background for search popup
        frame.render_widget(Clear, search_area);

        // Search block with highlight
        let search_block = Block::default()
            .title("Search Servers")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let search_text = Paragraph::new(app.search_query.clone())
            .block(search_block)
            .style(Style::default().fg(Color::White));

        frame.render_widget(search_text, search_area);

        // Set cursor position for search input
        frame.set_cursor(
            search_area.x + 1 + app.search_cursor as u16,
            search_area.y + 1,
        );
    }
}
