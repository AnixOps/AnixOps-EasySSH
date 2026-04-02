//! Group Dialog
//!
//! Form dialog for creating or editing groups.
//! Fields:
//! - Name (required)
//! - Color (hex color code)
//!
//! Styled with theme support.

use super::{Dialog, DialogResult, GroupData};
use crate::theme::ColorPalette;
use crate::ui::dialogs::handle_text_input;
use crossterm::event::{KeyCode, KeyEvent};
use easyssh_core::GroupRecord;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Fields in the group dialog
#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Name,
    Color,
}

/// Group creation/editing dialog
pub struct GroupDialog {
    title: String,
    data: GroupData,
    focused_field: Field,
    cursor_name: usize,
    cursor_color: usize,
    preset_colors: Vec<&'static str>,
}

impl GroupDialog {
    pub fn new(title: String, group: Option<GroupRecord>) -> Self {
        let data = if let Some(group) = group {
            GroupData {
                id: group.id,
                name: group.name,
                color: "#4A90D9".to_string(), // Default color
            }
        } else {
            GroupData::default()
        };

        Self {
            title,
            data,
            focused_field: Field::Name,
            cursor_name: data.name.len(),
            cursor_color: data.color.len(),
            preset_colors: vec![
                "#EF4444", // Red
                "#F97316", // Orange
                "#F59E0B", // Amber
                "#84CC16", // Lime
                "#22C55E", // Green
                "#10B981", // Emerald
                "#14B8A6", // Teal
                "#06B6D4", // Cyan
                "#0EA5E9", // Sky
                "#3B82F6", // Blue
                "#6366F1", // Indigo
                "#8B5CF6", // Violet
                "#A855F7", // Purple
                "#D946EF", // Fuchsia
                "#EC4899", // Pink
                "#F43F5E", // Rose
                "#6B7280", // Gray
            ],
        }
    }

    fn is_valid_color(&self) -> bool {
        let color = &self.data.color;
        color.starts_with('#')
            && (color.len() == 4 || color.len() == 7)
            && color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }
}

impl Dialog for GroupDialog {
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        // Handle dialog navigation
        match key.code {
            KeyCode::Tab => {
                self.focused_field = match self.focused_field {
                    Field::Name => Field::Color,
                    Field::Color => Field::Name,
                };
                return DialogResult::Continue;
            }
            KeyCode::BackTab => {
                self.focused_field = match self.focused_field {
                    Field::Name => Field::Color,
                    Field::Color => Field::Name,
                };
                return DialogResult::Continue;
            }
            KeyCode::Esc => return DialogResult::Cancel,
            KeyCode::Enter => {
                if self.is_valid() {
                    return DialogResult::GroupData(self.data.clone());
                }
            }
            _ => {}
        }

        // Handle field-specific input
        match self.focused_field {
            Field::Name => {
                handle_text_input(key, &mut self.data.name, &mut self.cursor_name);
            }
            Field::Color => {
                // Check for number keys to select preset colors
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let idx =
                            (c.to_digit(10).unwrap() as usize).min(self.preset_colors.len() - 1);
                        if let Some(&color) = self.preset_colors.get(idx) {
                            self.data.color = color.to_string();
                            self.cursor_color = self.data.color.len();
                        }
                    }
                    _ => {
                        handle_text_input(key, &mut self.data.color, &mut self.cursor_color);
                    }
                }
            }
        }

        DialogResult::Continue
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &ColorPalette) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_secondary))
            .style(Style::default().bg(theme.bg_dialog));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Name
                Constraint::Length(6), // Color + presets
                Constraint::Length(2), // Help
            ])
            .split(inner);

        // Helper function to render field
        fn render_field(
            frame: &mut Frame,
            area: Rect,
            label: &str,
            value: &str,
            is_focused: bool,
            cursor_pos: usize,
            theme: &ColorPalette,
            is_valid: bool,
        ) {
            let label_style = Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme.fg_secondary);

            let value_style = if is_focused {
                Style::default()
                    .fg(theme.fg_primary)
                    .bg(theme.bg_highlight)
            } else if !is_valid && value.is_empty() {
                Style::default().fg(theme.accent_error)
            } else {
                Style::default().fg(theme.fg_primary)
            };

            let text = Line::from(vec![
                Span::styled(label, label_style),
                Span::raw(" "),
                Span::styled(value, value_style),
            ]);

            let para = Paragraph::new(text);
            frame.render_widget(para, area);

            if is_focused {
                frame.set_cursor(area.x + label.len() as u16 + 1 + cursor_pos as u16, area.y);
            }
        }

        // Name field
        let name_valid = !self.data.name.is_empty();
        render_field(
            frame,
            chunks[0],
            "Name:",
            &self.data.name,
            self.focused_field == Field::Name,
            self.cursor_name,
            theme,
            name_valid,
        );

        // Color field with preview
        let color_style = if self.focused_field == Field::Color {
            Style::default()
                .fg(theme.fg_primary)
                .bg(theme.bg_highlight)
        } else {
            Style::default().fg(theme.fg_primary)
        };

        let color_rgb = if self.is_valid_color() {
            parse_hex_color(&self.data.color)
        } else {
            (128, 128, 128)
        };

        let color_preview = Span::styled(
            "  ",
            Style::default().bg(Color::Rgb(color_rgb.0, color_rgb.1, color_rgb.2)),
        );

        let color_line = Line::from(vec![
            Span::styled("Color:", Style::default().add_modifier(Modifier::BOLD).fg(theme.fg_secondary)),
            Span::styled(format!(" {} ", self.data.color), color_style),
            color_preview,
        ]);

        let color_para = Paragraph::new(color_line);
        frame.render_widget(color_para, chunks[1]);

        // Preset colors - show first 10 with numbers
        let mut preset_spans = vec![
            Span::styled("Presets: ", Style::default().fg(theme.fg_secondary)),
        ];

        for (idx, &color) in self.preset_colors.iter().take(10).enumerate() {
            let rgb = parse_hex_color(color);
            let is_selected = self.data.color == color;

            let style = if is_selected {
                Style::default()
                    .bg(Color::Rgb(rgb.0, rgb.1, rgb.2))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().bg(Color::Rgb(rgb.0, rgb.1, rgb.2)).fg(Color::White)
            };

            preset_spans.push(Span::styled(format!(" {}", idx), style));
        }

        let presets_para = Paragraph::new(Line::from(preset_spans));
        frame.render_widget(presets_para, chunks[1].offset(0, 1));

        // Help text with theme
        let help_style = if self.is_valid() {
            Style::default().fg(theme.fg_muted)
        } else {
            Style::default().fg(theme.accent_warning)
        };

        let help_text = if self.is_valid() {
            "0-9: Select preset  |  Enter: Save  |  Esc: Cancel"
        } else {
            "⚠ Group name is required"
        };

        let help_para = Paragraph::new(help_text).style(help_style);
        frame.render_widget(help_para, chunks[2]);
    }

    fn is_valid(&self) -> bool {
        !self.data.name.trim().is_empty() && self.is_valid_color()
    }

    fn title(&self) -> &str {
        &self.title
    }
}

/// Parse hex color to RGB tuple
fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    if hex.len() >= 7 && hex.starts_with('#') {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128);
        (r, g, b)
    } else if hex.len() >= 4 && hex.starts_with('#') {
        // Short hex format #RGB
        let r = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[3..4].repeat(2), 16).unwrap_or(128);
        (r, g, b)
    } else {
        (128, 128, 128)
    }
}
