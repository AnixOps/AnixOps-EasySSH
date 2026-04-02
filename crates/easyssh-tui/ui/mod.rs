//! UI Module
//!
//! This module provides all UI components:
//! - Layout management
//! - Server list rendering
//! - Sidebar rendering
//! - Detail panel rendering
//! - Dialog rendering

pub mod detail_panel;
pub mod dialogs;
pub mod layout;
pub mod server_list;
pub mod sidebar;

use crate::app::App;
use ratatui::{style::Style, widgets::Clear, Frame};

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

        // Clear background with theme color
        let palette = app.palette();
        frame.render_widget(
            ratatui::widgets::Block::default().style(Style::default().bg(palette.bg_primary)),
            frame.size(),
        );

        // Render main layout areas
        let areas = self.layout.calculate_areas(frame.size());

        // Render sidebar (groups) with theme
        sidebar::render(frame, areas.sidebar, app, palette);

        // Render server list with virtual scrolling and theme
        server_list::render(frame, areas.server_list, app, palette);

        // Render detail panel with theme
        detail_panel::render(frame, areas.detail_panel, app, palette);

        // Render status bar with theme
        self.render_status_bar(frame, areas.status_bar, app, palette);

        // Render dialog overlay if present
        if let Some(dialog) = &mut app.dialog {
            let dialog_area = self.layout.calculate_dialog_area(frame.size());
            dialog.render(frame, dialog_area, palette);
        }

        // Render search overlay if in search mode
        if app.view_mode == crate::app::ViewMode::Search {
            self.render_search_bar(frame, app, palette);
        }
    }


    fn render_status_bar(&self, frame: &mut Frame, area: ratatui::layout::Rect, app: &App, palette: &crate::theme::ColorPalette) {
        use ratatui::style::{Color, Style, Modifier};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let version = env!("CARGO_PKG_VERSION");
        let server_count = app.get_filtered_server_count();
        let total_count = app.get_total_server_count();
        let theme_name = &app.theme.name;

        // Build status parts
        let mut status_spans = vec![
            Span::styled(
                format!(" EasySSH Lite v{} ", version),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("| {} of {} servers ", server_count, total_count),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("| Theme: {} ", theme_name),
                Style::default().fg(Color::White),
            ),
        ];

        // Add search indicator or status message
        if app.view_mode == crate::app::ViewMode::Search {
            status_spans.push(Span::styled(
                format!("| Search: {} ", app.search_query),
                Style::default()
                    .fg(palette.accent_info)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            status_spans.push(Span::styled(
                format!("| {} ", app.status_message),
                Style::default().fg(Color::White),
            ));
        }

        // Add keyboard hints
        status_spans.push(Span::styled(
            " | ? for help ",
            Style::default().fg(Color::Gray),
        ));

        let status = Paragraph::new(Line::from(status_spans))
            .style(Style::default().bg(palette.bg_status_bar));

        frame.render_widget(status, area);
    }

    fn render_search_bar(&self, frame: &mut Frame, app: &App, palette: &crate::theme::ColorPalette) {
        use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Clear, Paragraph};

        let area = frame.size();
        let search_width = (area.width * 2) / 3;
        let search_height = 5;
        let search_area = Rect {
            x: (area.width - search_width) / 2,
            y: area.height / 3,
            width: search_width,
            height: search_height,
        };

        // Clear background for search popup
        frame.render_widget(Clear, search_area);

        // Search block with theme colors
        let search_block = Block::default()
            .title(" Search Servers ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent_info))
            .style(Style::default().bg(palette.bg_dialog));

        let inner = search_block.inner(search_area);

        // Create search content
        let search_text = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Filter: "),
                Span::styled(
                    &app.search_query,
                    Style::default()
                        .fg(palette.fg_primary)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::raw(" "),
                Span::styled(
                    "█",
                    Style::default().fg(palette.accent_primary),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("  Found: {} matches", app.get_filtered_server_count()),
                    Style::default().fg(palette.fg_muted),
                ),
            ]),
        ])
        .block(search_block);

        frame.render_widget(search_text, search_area);

        // Set cursor position for search input
        frame.set_cursor(
            inner.x + 10 + app.search_cursor as u16,
            inner.y + 1,
        );
    }
}
