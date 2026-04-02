//! Sidebar Component
//!
//! Renders the left sidebar with the groups list.
//! Includes:
//! - "All" pseudo-group showing all servers
//! - Individual groups with server counts
//! - Visual indication of selected group

use crate::app::{App, Focus};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Determine border style based on focus
    let is_focused = app.focus == Focus::Sidebar;
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .title(" Groups ")
        .borders(Borders::ALL)
        .border_style(border_style);

    // Build group list items
    let mut items: Vec<ListItem> = Vec::new();

    // Add "All" pseudo-group
    let all_count = app.get_total_server_count();
    let all_selected = app.selected_group == 0;
    let all_style = if all_selected && is_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if all_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let all_symbol = if all_selected { "▶ " } else { "  " };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("{}{} ({})", all_symbol, "All", all_count),
            all_style,
        ),
    ])));

    // Add groups with server counts
    for (index, group) in app.groups.iter().enumerate() {
        let list_index = index + 1; // +1 because "All" is at index 0
        let count = app.get_group_server_count(&group.id);
        let is_selected = app.selected_group == list_index;

        let style = if is_selected && is_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let symbol = if is_selected { "> " } else { "  " };
        // Use default color since GroupRecord doesn't have color field
        let group_color = Color::Cyan;

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{}{} ", symbol, group.name),
                style.fg(group_color),
            ),
            Span::styled(
                format!("({})", count),
                style.fg(Color::Gray),
            ),
        ])));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Convert hex color to closest ANSI color
fn group_color_to_ansi(hex: &str) -> Color {
    // Parse hex color and convert to RGB
    if hex.len() >= 7 && hex.starts_with('#') {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}
