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

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::ColorPalette) {
    // Determine border style based on focus
    let is_focused = app.focus == Focus::Sidebar;
    let border_style = if is_focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border_unfocused)
    };

    let block = Block::default()
        .title(" Groups ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg_primary));

    // Build group list items
    let mut items: Vec<ListItem> = Vec::new();

    // Add "All" pseudo-group
    let all_count = app.get_total_server_count();
    let all_selected = app.selected_group == 0;
    let all_style = if all_selected && is_focused {
        Style::default()
            .bg(theme.bg_selected)
            .fg(theme.fg_selected)
            .add_modifier(Modifier::BOLD)
    } else if all_selected {
        Style::default()
            .fg(theme.accent_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg_primary)
    };

    let all_symbol = if all_selected { "▶" } else { " " };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(all_symbol, Style::default().fg(theme.accent_info)),
        Span::styled(format!(" All "), all_style),
        Span::styled(format!("({})", all_count), Style::default().fg(theme.fg_muted)),
    ])));

    // Add groups with server counts
    for (index, group) in app.groups.iter().enumerate() {
        let list_index = index + 1; // +1 because "All" is at index 0
        let count = app.get_group_server_count(&group.id);
        let is_selected = app.selected_group == list_index;

        let style = if is_selected && is_focused {
            Style::default()
                .bg(theme.bg_selected)
                .fg(theme.fg_selected)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg_primary)
        };

        let symbol = if is_selected { "▶" } else { " " };
        let group_color = theme.accent_secondary;

        items.push(ListItem::new(Line::from(vec![
            Span::styled(symbol, Style::default().fg(theme.accent_info)),
            Span::styled(format!(" {} ", group.name), style.fg(group_color)),
            Span::styled(format!("({})", count), Style::default().fg(theme.fg_muted)),
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
