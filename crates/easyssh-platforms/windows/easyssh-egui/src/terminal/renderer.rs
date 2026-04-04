//! Terminal Renderer - egui Painting Logic
//!
//! Handles efficient rendering of terminal content using egui painter:
//! - Cell-by-cell painting with styling
//! - Cursor rendering with blink
//! - Selection highlighting
//! - Search result highlighting
//! - Optimized batch painting

use std::time::Instant;

use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Ui, Vec2};

use super::buffer::{Cell, ColorScheme, TermLine, TerminalBuffer};
use super::{CursorShape, SearchMatch, TerminalConfig};

/// Terminal renderer for egui painting
pub struct TerminalRenderer {
    /// Font for terminal text
    font: FontId,
    /// Cell size (width, height) in pixels
    cell_size: Vec2,
    /// Color scheme
    colors: ColorScheme,
    /// Cursor shape
    cursor_shape: CursorShape,
    /// Whether cursor should blink
    cursor_blink: bool,
}

impl TerminalRenderer {
    /// Create renderer from configuration
    pub fn new(config: &TerminalConfig) -> Self {
        let font = FontId::monospace(config.font.size);

        // Calculate cell size based on font metrics
        let cell_size = Self::calculate_cell_size(config.font.size, config.font.line_height);

        Self {
            font,
            cell_size,
            colors: config.colors.clone(),
            cursor_shape: config.cursor.shape,
            cursor_blink: config.cursor.blink,
        }
    }

    /// Calculate cell size from font metrics
    fn calculate_cell_size(font_size: f32, line_height: f32) -> Vec2 {
        // Typical monospace font metrics
        // Width is approximately 0.6 * font_size
        // Height is font_size * line_height
        Vec2::new(font_size * 0.6, font_size * line_height)
    }

    /// Get cell size
    pub fn cell_size(&self) -> Vec2 {
        self.cell_size
    }

    /// Render terminal content
    pub fn render(
        &self,
        ui: &mut Ui,
        buffer: &TerminalBuffer,
        scroll_start: usize,
        visible_rows: usize,
        focused: bool,
        selection: &Option<super::view::Selection>,
        cursor_pos: (usize, usize),
        search_results: &[SearchMatch],
        search_result_idx: usize,
        last_render: Instant,
    ) {
        let painter = ui.painter();
        let rect = ui.available_rect_before_wrap();

        // Calculate content area (with padding)
        let content_rect = Rect::from_min_size(
            rect.min + Vec2::new(4.0, 4.0),
            rect.size() - Vec2::new(8.0, 8.0),
        );

        // Get visible lines
        let visible_lines = buffer.get_visible_lines(scroll_start, visible_rows);

        // Render lines
        for (row_idx, line) in visible_lines.iter().enumerate() {
            let actual_row = scroll_start + row_idx;
            self.render_line(
                painter,
                content_rect,
                row_idx,
                line,
                selection,
                actual_row,
                search_results,
            );
        }

        // Render cursor if focused
        if focused {
            self.render_cursor(
                painter,
                content_rect,
                cursor_pos,
                scroll_start,
                last_render,
            );
        }

        // Render search highlights
        self.render_search_highlights(
            painter,
            content_rect,
            scroll_start,
            search_results,
            search_result_idx,
        );
    }

    /// Render a single line
    fn render_line(
        &self,
        painter: &Painter,
        content_rect: Rect,
        row_idx: usize,
        line: &TermLine,
        selection: &Option<super::view::Selection>,
        actual_row: usize,
        search_results: &[SearchMatch],
    ) {
        let y = content_rect.min.y + (row_idx as f32 * self.cell_size.y);

        // Render cells
        let cells = line.cells();
        for (col_idx, cell) in cells.iter().enumerate() {
            let x = content_rect.min.x + (col_idx as f32 * self.cell_size.x);

            // Check if cell is in selection
            let is_selected = selection
                .map(|s| s.contains(actual_row, col_idx))
                .unwrap_or(false);

            // Check if cell matches search
            let is_search_match = search_results
                .iter()
                .any(|m| m.line == actual_row && col_idx >= m.cols.0 && col_idx < m.cols.1);

            // Render cell
            self.render_cell(
                painter,
                Pos2::new(x, y),
                cell,
                is_selected,
                is_search_match,
            );
        }
    }

    /// Render a single cell
    fn render_cell(
        &self,
        painter: &Painter,
        pos: Pos2,
        cell: &Cell,
        is_selected: bool,
        is_search_match: bool,
    ) {
        let cell_rect = Rect::from_min_size(pos, self.cell_size);

        // Get effective colors
        let (fg, bg) = cell.style.effective_colors();

        // Selection overrides background
        let bg_color = if is_selected {
            Color32::from_rgba_unmultiplied(100, 150, 255, 100)
        } else if is_search_match {
            Color32::from_rgba_unmultiplied(255, 200, 50, 80)
        } else {
            bg
        };

        // Paint background
        painter.rect_filled(cell_rect, 0.0, bg_color);

        // Paint character if not empty
        if cell.char != ' ' {
            // Apply search highlight to foreground
            let fg_color = if is_search_match {
                Color32::from_rgb(255, 255, 200)
            } else {
                fg
            };

            // Paint text
            painter.text(
                cell_rect.left_top() + Vec2::new(0.0, 2.0),
                egui::Align2::LEFT_TOP,
                cell.char.to_string(),
                self.font.clone(),
                fg_color,
            );

            // Paint underline if needed
            if cell.style.underline {
                painter.line_segment(
                    [
                        cell_rect.left_bottom() - Vec2::new(0.0, 2.0),
                        cell_rect.right_bottom() - Vec2::new(0.0, 2.0),
                    ],
                    Stroke::new(1.0, fg_color),
                );
            }
        }
    }

    /// Render cursor
    fn render_cursor(
        &self,
        painter: &Painter,
        content_rect: Rect,
        cursor_pos: (usize, usize),
        scroll_start: usize,
        last_render: Instant,
    ) {
        // Calculate cursor position relative to visible area
        let cursor_row = cursor_pos.1;
        if cursor_row < scroll_start {
            return; // Cursor is above visible area
        }

        let visible_row = cursor_row - scroll_start;
        let x = content_rect.min.x + (cursor_pos.0 as f32 * self.cell_size.x);
        let y = content_rect.min.y + (visible_row as f32 * self.cell_size.y);

        // Handle cursor blink
        let cursor_visible = if self.cursor_blink {
            // Blink every 500ms
            let elapsed = last_render.elapsed().as_millis() as u32;
            (elapsed / 500) % 2 == 0
        } else {
            true
        };

        if !cursor_visible {
            return;
        }

        let cursor_rect = Rect::from_min_size(Pos2::new(x, y), self.cell_size);

        // Render cursor based on shape
        match self.cursor_shape {
            CursorShape::Block => {
                // Full block cursor
                painter.rect_filled(
                    cursor_rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(200, 200, 200, 150),
                );
            }
            CursorShape::Underline => {
                // Underline cursor
                painter.line_segment(
                    [
                        cursor_rect.left_bottom() - Vec2::new(0.0, 2.0),
                        cursor_rect.right_bottom() - Vec2::new(0.0, 2.0),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(200, 200, 200)),
                );
            }
            CursorShape::Bar => {
                // Vertical bar cursor
                painter.line_segment(
                    [
                        cursor_rect.left_top(),
                        cursor_rect.left_bottom(),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(200, 200, 200)),
                );
            }
        }
    }

    /// Render search result highlights
    fn render_search_highlights(
        &self,
        painter: &Painter,
        content_rect: Rect,
        scroll_start: usize,
        search_results: &[SearchMatch],
        current_idx: usize,
    ) {
        // Render current search result with stronger highlight
        for (idx, result) in search_results.iter().enumerate() {
            let is_current = idx == current_idx;

            // Check if result is in visible range
            if result.line < scroll_start {
                continue;
            }

            let visible_row = result.line - scroll_start;
            let y = content_rect.min.y + (visible_row as f32 * self.cell_size.y);

            let start_x = content_rect.min.x + (result.cols.0 as f32 * self.cell_size.x);
            let end_x = content_rect.min.x + (result.cols.1 as f32 * self.cell_size.x);

            let highlight_rect = Rect::from_min_size(
                Pos2::new(start_x, y),
                Vec2::new(end_x - start_x, self.cell_size.y),
            );

            let color = if is_current {
                Color32::from_rgba_unmultiplied(255, 200, 50, 120)
            } else {
                Color32::from_rgba_unmultiplied(255, 200, 50, 60)
            };

            painter.rect_filled(highlight_rect, 0.0, color);

            // Add border for current match
            if is_current {
                painter.rect_stroke(
                    highlight_rect,
                    0.0,
                    Stroke::new(2.0, Color32::from_rgb(255, 180, 50)),
                );
            }
        }
    }
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        Self::new(&TerminalConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let config = TerminalConfig::default();
        let renderer = TerminalRenderer::new(&config);
        assert!(renderer.cell_size.x > 0.0);
        assert!(renderer.cell_size.y > 0.0);
    }

    #[test]
    fn test_cell_size_calculation() {
        let cell_size = TerminalRenderer::calculate_cell_size(14.0, 1.2);
        assert_eq!(cell_size.x, 14.0 * 0.6); // 8.4
        assert_eq!(cell_size.y, 14.0 * 1.2); // 16.8
    }

    #[test]
    fn test_cursor_shapes() {
        let config = TerminalConfig {
            cursor: super::super::CursorConfig {
                shape: CursorShape::Block,
                blink: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let renderer = TerminalRenderer::new(&config);
        assert_eq!(renderer.cursor_shape, CursorShape::Block);
        assert!(renderer.cursor_blink);
    }
}