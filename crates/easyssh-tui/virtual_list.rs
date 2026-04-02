//! Virtual List Component
//!
//! High-performance list rendering for large datasets.
//! Only renders visible items, enabling smooth navigation through thousands of entries.
//! Inspired by ranger's file browser rendering approach.

use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

/// Virtual list configuration
#[derive(Debug, Clone)]
pub struct VirtualListConfig {
    /// Height of each row
    pub row_height: u16,
    /// Header height
    pub header_height: u16,
    /// Maximum visible items to render
    pub max_visible_items: usize,
}

impl Default for VirtualListConfig {
    fn default() -> Self {
        Self {
            row_height: 1,
            header_height: 1,
            max_visible_items: 100,
        }
    }
}

/// Virtual list state
#[derive(Debug)]
pub struct VirtualListState {
    /// Total number of items
    pub total_items: usize,
    /// Currently selected index
    pub selected: usize,
    /// First visible item index (scroll offset)
    pub scroll_offset: usize,
    /// Number of visible items
    pub visible_count: usize,
    /// Table state for ratatui
    pub table_state: TableState,
}

impl VirtualListState {
    pub fn new(total_items: usize) -> Self {
        let mut table_state = TableState::default();
        if total_items > 0 {
            table_state.select(Some(0));
        }

        Self {
            total_items,
            selected: 0,
            scroll_offset: 0,
            visible_count: 0,
            table_state,
        }
    }

    /// Update the view based on container height
    pub fn update_view(&mut self, available_height: u16, row_height: u16) {
        let max_visible = (available_height as usize / row_height as usize).saturating_sub(1);
        self.visible_count = max_visible.min(self.total_items);

        // Adjust scroll offset to keep selection visible
        if self.selected < self.scroll_offset {
            // Selection is above visible area - scroll up
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_count {
            // Selection is below visible area - scroll down
            self.scroll_offset = self.selected.saturating_sub(self.visible_count - 1);
        }

        // Ensure scroll offset doesn't exceed bounds
        let max_offset = self.total_items.saturating_sub(self.visible_count);
        self.scroll_offset = self.scroll_offset.min(max_offset);

        // Update table state
        self.table_state.select(Some(self.selected - self.scroll_offset));
    }

    /// Navigate up
    pub fn navigate_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Navigate down
    pub fn navigate_down(&mut self) {
        if self.selected + 1 < self.total_items {
            self.selected += 1;
        }
    }

    /// Navigate to first item
    pub fn go_to_first(&mut self) {
        self.selected = 0;
    }

    /// Navigate to last item
    pub fn go_to_last(&mut self) {
        self.selected = self.total_items.saturating_sub(1);
    }

    /// Navigate page up
    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(self.visible_count.max(1));
    }

    /// Navigate page down
    pub fn page_down(&mut self) {
        self.selected = (self.selected + self.visible_count).min(self.total_items.saturating_sub(1));
    }

    /// Get visible range
    pub fn visible_range(&self) -> (usize, usize) {
        let end = (self.scroll_offset + self.visible_count).min(self.total_items);
        (self.scroll_offset, end)
    }

    /// Get the actual item index from a visible position
    pub fn get_item_index(&self, visible_index: usize) -> Option<usize> {
        let actual_index = self.scroll_offset + visible_index;
        if actual_index < self.total_items {
            Some(actual_index)
        } else {
            None
        }
    }
}

/// Server list item data
#[derive(Debug, Clone)]
pub struct ServerListItem {
    pub index: usize,
    pub name: String,
    pub connection: String,
    pub group: String,
    pub status: String,
    pub is_selected: bool,
}

/// Render a virtual server list
pub fn render_virtual_server_list(
    frame: &mut Frame,
    area: Rect,
    items: &[ServerListItem],
    state: &mut VirtualListState,
    is_focused: bool,
    theme: &crate::theme::ColorPalette,
) {
    // Calculate visible count based on area height
    let available_height = area.height.saturating_sub(2); // Account for borders
    state.update_view(available_height, 1);

    let (start, end) = state.visible_range();

    // Build visible rows only
    let mut rows: Vec<Row> = Vec::with_capacity(end - start);

    for (idx, item) in items.iter().enumerate().skip(start).take(end - start) {
        let is_selected = item.is_selected;

        // Style based on selection and focus
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

        // Status symbol and color
        let status_symbol = status_to_symbol(&item.status);
        let status_color = theme.server_status_color(&item.status);

        // Selection indicator
        let indicator = if is_selected { "▶" } else { " " };

        let cells = vec![
            Cell::from(Line::from(vec![
                Span::styled(indicator, style),
                Span::styled(status_symbol, style.fg(status_color)),
            ])),
            Cell::from(item.name.clone()).style(style),
            Cell::from(item.connection.clone()).style(style),
            Cell::from(item.group.clone())
                .style(style.fg(theme.fg_muted)),
        ];

        rows.push(Row::new(cells));
    }

    // Empty state
    if items.is_empty() {
        rows.push(Row::new(vec![
            Cell::from(""),
            Cell::from("No servers configured").style(Style::default().fg(theme.fg_muted)),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    // Create table with scroll indicator in title if needed
    let title = if state.total_items > state.visible_count {
        let scroll_pct = if state.total_items > 0 {
            (state.scroll_offset * 100 / state.total_items)
        } else {
            0
        };
        format!(" Servers [{}%] ", scroll_pct)
    } else {
        " Servers ".to_string()
    };

    let border_style = if is_focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border_unfocused)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),    // Indicator + status
            Constraint::Length(25),   // Name
            Constraint::Length(35),   // Connection
            Constraint::Length(15),   // Group
        ],
    )
    .header(
        Row::new(vec!["", "Name", "Connection", "Group"])
            .style(Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme.fg_secondary)),
    )
    .block(block);

    frame.render_stateful_widget(table, area, &mut state.table_state);

    // Render scroll indicators if needed
    if state.total_items > state.visible_count {
        // Top scroll indicator
        if state.scroll_offset > 0 {
            let indicator = Span::styled("▲ more ▲", Style::default().fg(theme.accent_info));
            let indicator_area = Rect {
                x: area.x + area.width - 8,
                y: area.y + 1,
                width: 6,
                height: 1,
            };
            frame.render_widget(
                ratatui::widgets::Paragraph::new(indicator),
                indicator_area,
            );
        }

        // Bottom scroll indicator
        if end < state.total_items {
            let indicator = Span::styled("▼ more ▼", Style::default().fg(theme.accent_info));
            let indicator_area = Rect {
                x: area.x + area.width - 8,
                y: area.y + area.height - 2,
                width: 6,
                height: 1,
            };
            frame.render_widget(
                ratatui::widgets::Paragraph::new(indicator),
                indicator_area,
            );
        }
    }
}

/// Convert server status to symbol
fn status_to_symbol(status: &str) -> &'static str {
    match status {
        "online" => "●",
        "offline" => "○",
        "error" => "✗",
        "connecting" => "⟳",
        _ => "?",
    }
}

/// Sidebar group item
#[derive(Debug, Clone)]
pub struct GroupListItem {
    pub name: String,
    pub count: usize,
    pub is_selected: bool,
}

/// Render virtual group list (sidebar)
pub fn render_virtual_group_list(
    frame: &mut Frame,
    area: Rect,
    items: &[GroupListItem],
    selected: usize,
    is_focused: bool,
    theme: &crate::theme::ColorPalette,
) {
    let available_height = area.height.saturating_sub(2) as usize;

    // Calculate scroll offset
    let scroll_offset = if selected >= available_height {
        selected.saturating_sub(available_height / 2)
    } else {
        0
    };

    let visible_end = (scroll_offset + available_height).min(items.len());

    let border_style = if is_focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border_unfocused)
    };

    let block = Block::default()
        .title(" Groups ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let mut list_items: Vec<Line> = Vec::new();

    // Add "All" group at top
    let all_selected = selected == 0;
    let total_count: usize = items.iter().map(|i| i.count).sum();

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
    list_items.push(Line::from(vec![
        Span::styled(format!("{}{} ", all_symbol, "All"), all_style),
        Span::styled(format!("({})", total_count), Style::default().fg(theme.fg_muted)),
    ]));

    // Add visible groups
    for (idx, item) in items.iter().enumerate().skip(scroll_offset).take(visible_end - scroll_offset) {
        let list_idx = idx + 1; // +1 for "All"
        let is_selected = selected == list_idx;

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
        list_items.push(Line::from(vec![
            Span::styled(format!("{}{} ", symbol, item.name), style),
            Span::styled(format!("({})", item.count), Style::default().fg(theme.fg_muted)),
        ]));
    }

    use ratatui::widgets::Paragraph;
    let paragraph = Paragraph::new(list_items).block(block);
    frame.render_widget(paragraph, area);

    // Scroll indicator
    if items.len() > available_height {
        let has_more_below = visible_end < items.len();
        if has_more_below {
            let indicator = Span::styled("▼", Style::default().fg(theme.accent_info));
            let indicator_area = Rect {
                x: area.x + area.width - 3,
                y: area.y + area.height - 2,
                width: 1,
                height: 1,
            };
            frame.render_widget(Paragraph::new(indicator), indicator_area);
        }
    }
}
