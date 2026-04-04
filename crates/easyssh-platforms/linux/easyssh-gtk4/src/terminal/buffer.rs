//! Terminal Buffer for GTK4
//!
//! Provides the text buffer management for terminal output with:
//! - FIFO eviction for scroll limit
//! - Search capabilities
//! - ANSI escape sequence handling
//! - Text styling with tags
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.2)
//!
//! - Scroll buffer max lines: 10000 (Standard version)
//! - FIFO eviction when exceeding limit
//! - Search MUST NOT block output processing

use gtk4::prelude::*;
use gtk4::{TextBuffer, TextTag, TextTagTable, TextIter};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::style::TerminalStyle;

/// Maximum lines in terminal buffer (per SYSTEM_INVARIANTS.md Section 8.2).
const DEFAULT_MAX_LINES: usize = 10000;

/// Search match result with position information.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Line number (0-based)
    pub line_number: usize,
    /// Start character offset in line
    pub start_offset: usize,
    /// End character offset in line
    pub end_offset: usize,
    /// Matched text content
    pub matched_text: String,
    /// Line content for context
    pub line_content: String,
}

impl SearchMatch {
    /// Create a new search match.
    pub fn new(
        line_number: usize,
        start_offset: usize,
        end_offset: usize,
        matched_text: &str,
        line_content: &str,
    ) -> Self {
        Self {
            line_number,
            start_offset,
            end_offset,
            matched_text: matched_text.to_string(),
            line_content: line_content.to_string(),
        }
    }
}

/// Text style options for buffer content.
#[derive(Debug, Clone, Copy)]
pub enum TextStyle {
    /// Normal text (default style)
    Normal,
    /// Bold text
    Bold,
    /// Italic text
    Italic,
    /// Underlined text
    Underline,
    /// ANSI foreground color (0-255)
    ForegroundColor(u8),
    /// ANSI background color (0-255)
    BackgroundColor(u8),
    /// Combined bold and color
    BoldColor(u8),
    /// Error style (red foreground)
    Error,
    /// Success style (green foreground)
    Success,
    /// Warning style (yellow foreground)
    Warning,
}

/// Line metadata for tracking.
#[derive(Debug, Clone)]
struct LineMeta {
    /// Line content
    content: String,
    /// Timestamp when added (for debugging)
    added_at: std::time::Instant,
    /// Whether line is wrapped
    is_wrapped: bool,
}

/// Terminal buffer with FIFO eviction and search.
///
/// Manages the GTK4 TextBuffer with:
/// - Maximum line limit enforcement
/// - Text tag table for styling
/// - Search functionality
/// - ANSI color support
///
/// # Thread Safety
///
/// Uses RefCell for interior mutability within GTK4's single-threaded model.
pub struct TerminalBuffer {
    /// GTK4 text buffer
    buffer: TextBuffer,
    /// Text tag table for styling
    tag_table: TextTagTable,
    /// Maximum lines to retain
    max_lines: usize,
    /// Current style configuration
    style: TerminalStyle,
    /// Line metadata tracking (for search)
    line_queue: RefCell<VecDeque<LineMeta>>,
    /// Total lines evicted
    evicted_count: RefCell<u64>,
    /// Line start positions (for search optimization)
    line_starts: RefCell<Vec<usize>>,
}

impl TerminalBuffer {
    /// Create a new terminal buffer with default settings.
    pub fn new() -> Self {
        Self::with_style(TerminalStyle::default())
    }

    /// Create a new terminal buffer with custom style.
    pub fn with_style(style: TerminalStyle) -> Self {
        let tag_table = TextTagTable::new();
        let buffer = TextBuffer::new(Some(&tag_table));

        // Create default text tags
        Self::create_default_tags(&tag_table, &style);

        Self {
            buffer,
            tag_table,
            max_lines: DEFAULT_MAX_LINES,
            style,
            line_queue: RefCell::new(VecDeque::with_capacity(DEFAULT_MAX_LINES)),
            evicted_count: RefCell::new(0),
            line_starts: RefCell::new(Vec::with_capacity(DEFAULT_MAX_LINES)),
        }
    }

    /// Create a buffer with custom max lines.
    pub fn with_max_lines(max_lines: usize) -> Self {
        let mut buffer = Self::new();
        buffer.max_lines = max_lines;
        buffer
    }

    /// Create default text tags for styling.
    fn create_default_tags(tag_table: &TextTagTable, style: &TerminalStyle) {
        // Bold tag
        let bold_tag = TextTag::new(Some("bold"));
        bold_tag.set_weight(700); // PANGO_WEIGHT_BOLD
        tag_table.add(&bold_tag);

        // Italic tag
        let italic_tag = TextTag::new(Some("italic"));
        italic_tag.set_style(gtk4::pango::Style::Italic);
        tag_table.add(&italic_tag);

        // Underline tag
        let underline_tag = TextTag::new(Some("underline"));
        underline_tag.set_underline(gtk4::pango::Underline::Single);
        tag_table.add(&underline_tag);

        // Error tag (red foreground)
        let error_tag = TextTag::new(Some("error"));
        error_tag.set_foreground_rgba(Some(&style.colors[1])); // ANSI red
        tag_table.add(&error_tag);

        // Success tag (green foreground)
        let success_tag = TextTag::new(Some("success"));
        success_tag.set_foreground_rgba(Some(&style.colors[2])); // ANSI green
        tag_table.add(&success_tag);

        // Warning tag (yellow foreground)
        let warning_tag = TextTag::new(Some("warning"));
        warning_tag.set_foreground_rgba(Some(&style.colors[3])); // ANSI yellow
        tag_table.add(&warning_tag);

        // ANSI foreground colors (0-15)
        for i in 0..16 {
            let tag = TextTag::new(Some(&format!("fg-{}", i)));
            tag.set_foreground_rgba(Some(&style.colors[i]));
            tag_table.add(&tag);
        }

        // ANSI background colors (0-15)
        for i in 0..16 {
            let tag = TextTag::new(Some(&format!("bg-{}", i)));
            tag.set_background_rgba(Some(&style.colors[i]));
            tag_table.add(&tag);
        }

        // Selection highlight tag
        let selection_tag = TextTag::new(Some("search-highlight"));
        selection_tag.set_background_rgba(Some(&style.selection_bg));
        tag_table.add(&selection_tag);
    }

    /// Append text to the buffer.
    ///
    /// # Arguments
    ///
    /// * `text` - Text content to append
    /// * `style` - Optional text style to apply
    ///
    /// # Constraints
    ///
    /// - FIFO eviction when exceeding max_lines
    /// - Must not block UI thread
    pub fn append(&self, text: &str, style: Option<TextStyle>) {
        // Record line position before adding
        let start_pos = self.buffer.char_count();

        // Get end iterator
        let end_iter = self.buffer.end_iter();

        // Apply style if specified
        if let Some(s) = style {
            let tag_name = self.style_to_tag_name(s);
            if let Some(tag) = self.tag_table.lookup(&tag_name) {
                self.buffer.insert_with_tags(&end_iter, text, &[&tag]);
            } else {
                self.buffer.insert(&end_iter, text);
            }
        } else {
            self.buffer.insert(&end_iter, text);
        }

        // Track line metadata
        let mut line_queue = self.line_queue.borrow_mut();
        let mut line_starts = self.line_starts.borrow_mut();

        // Split text into lines and track each
        for line_content in text.lines() {
            line_queue.push_back(LineMeta {
                content: line_content.to_string(),
                added_at: std::time::Instant::now(),
                is_wrapped: false,
            });
            line_starts.push(start_pos);
        }

        // FIFO eviction check
        self.check_eviction(&mut line_queue, &mut line_starts);
    }

    /// Append text with ANSI escape sequence processing.
    ///
    /// Parses ANSI codes and applies appropriate styling.
    pub fn append_with_ansi(&self, text: &str) {
        // Simple ANSI parser for common sequences
        let mut current_style: Option<TextStyle> = None;
        let mut remaining = text;

        while !remaining.is_empty() {
            // Look for ANSI escape sequence
            if remaining.starts_with("\x1b[") {
                // Find the end of the sequence
                if let Some(end_pos) = remaining.find('m') {
                    let sequence = &remaining[2..end_pos];
                    current_style = self.parse_ansi_sequence(sequence);
                    remaining = &remaining[end_pos + 1..];
                } else {
                    // No complete sequence, append remaining
                    self.append(remaining, current_style);
                    break;
                }
            } else if let Some(seq_start) = remaining.find("\x1b[") {
                // Text before next sequence
                let text_before = &remaining[..seq_start];
                self.append(text_before, current_style);
                remaining = &remaining[seq_start..];
            } else {
                // No more sequences
                self.append(remaining, current_style);
                break;
            }
        }
    }

    /// Parse ANSI sequence to text style.
    fn parse_ansi_sequence(&self, sequence: &str) -> Option<TextStyle> {
        // Parse SGR (Select Graphic Rendition) parameters
        for param in sequence.split(';') {
            match param {
                "0" => return None, // Reset
                "1" => return Some(TextStyle::Bold),
                "3" => return Some(TextStyle::Italic),
                "4" => return Some(TextStyle::Underline),
                "30" => return Some(TextStyle::ForegroundColor(0)), // Black
                "31" => return Some(TextStyle::ForegroundColor(1)), // Red
                "32" => return Some(TextStyle::ForegroundColor(2)), // Green
                "33" => return Some(TextStyle::ForegroundColor(3)), // Yellow
                "34" => return Some(TextStyle::ForegroundColor(4)), // Blue
                "35" => return Some(TextStyle::ForegroundColor(5)), // Magenta
                "36" => return Some(TextStyle::ForegroundColor(6)), // Cyan
                "37" => return Some(TextStyle::ForegroundColor(7)), // White
                "38" | "48" => {
                    // Extended color (256 or RGB) - simplified handling
                    // Full implementation would parse 5;N or 2;R;G;B
                    return Some(TextStyle::ForegroundColor(15));
                }
                "90" => return Some(TextStyle::ForegroundColor(8)),  // Bright Black
                "91" => return Some(TextStyle::ForegroundColor(9)),  // Bright Red
                "92" => return Some(TextStyle::ForegroundColor(10)), // Bright Green
                "93" => return Some(TextStyle::ForegroundColor(11)), // Bright Yellow
                "94" => return Some(TextStyle::ForegroundColor(12)), // Bright Blue
                "95" => return Some(TextStyle::ForegroundColor(13)), // Bright Magenta
                "96" => return Some(TextStyle::ForegroundColor(14)), // Bright Cyan
                "97" => return Some(TextStyle::ForegroundColor(15)), // Bright White
                _ => continue,
            }
        }
        None
    }

    /// Convert TextStyle to tag name.
    fn style_to_tag_name(&self, style: TextStyle) -> String {
        match style {
            TextStyle::Normal => "normal",
            TextStyle::Bold => "bold",
            TextStyle::Italic => "italic",
            TextStyle::Underline => "underline",
            TextStyle::ForegroundColor(i) => return format!("fg-{}", i.min(15)),
            TextStyle::BackgroundColor(i) => return format!("bg-{}", i.min(15)),
            TextStyle::BoldColor(i) => return format!("fg-{}", i.min(15)),
            TextStyle::Error => "error",
            TextStyle::Success => "success",
            TextStyle::Warning => "warning",
        }.to_string()
    }

    /// Check and perform FIFO eviction.
    fn check_eviction(
        &self,
        line_queue: &mut VecDeque<LineMeta>,
        line_starts: &mut Vec<usize>,
    ) {
        while line_queue.len() > self.max_lines {
            // Remove oldest line
            line_queue.pop_front();

            // Remove from line starts
            if !line_starts.is_empty() {
                line_starts.remove(0);
            }

            // Increment evicted count
            *self.evicted_count.borrow_mut() += 1;

            // Trim buffer (remove first line)
            let start = self.buffer.start_iter();
            let first_line_end = self.buffer.iter_at_line(1);
            self.buffer.delete(&start, &first_line_end);
        }
    }

    /// Clear all buffer content.
    pub fn clear(&self) {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.delete(&start, &end);

        // Clear tracking
        self.line_queue.borrow_mut().clear();
        self.line_starts.borrow_mut().clear();
        *self.evicted_count.borrow_mut() = 0;
    }

    /// Get a specific line by number.
    ///
    /// # Arguments
    ///
    /// * `n` - Line number (0-based)
    ///
    /// # Returns
    ///
    /// The line content or None if out of range.
    pub fn get_line(&self, n: usize) -> Option<String> {
        let line_queue = self.line_queue.borrow();
        line_queue.get(n).map(|m| m.content.clone())
    }

    /// Search for text in the buffer.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Search pattern
    /// * `use_regex` - Whether to use regex matching
    ///
    /// # Returns
    ///
    /// Vector of search matches.
    ///
    /// # Constraints
    ///
    /// - Search MUST NOT block output processing (uses read-only operations)
    pub fn search(&self, pattern: &str, use_regex: bool) -> Vec<SearchMatch> {
        let line_queue = self.line_queue.borrow();
        let mut matches = Vec::new();

        if use_regex {
            // Regex search
            if let Ok(regex) = regex::Regex::new(pattern) {
                for (idx, meta) in line_queue.iter().enumerate() {
                    for cap in regex.find_iter(&meta.content) {
                        matches.push(SearchMatch::new(
                            idx,
                            cap.start(),
                            cap.end(),
                            &meta.content[cap.start()..cap.end()],
                            &meta.content,
                        ));
                    }
                }
            }
        } else {
            // Plain text search
            for (idx, meta) in line_queue.iter().enumerate() {
                let mut search_start = 0;
                while let Some(start) = meta.content[search_start..].find(pattern) {
                    let actual_start = search_start + start;
                    matches.push(SearchMatch::new(
                        idx,
                        actual_start,
                        actual_start + pattern.len(),
                        pattern,
                        &meta.content,
                    ));
                    search_start = actual_start + pattern.len();
                }
            }
        }

        matches
    }

    /// Highlight search results in the buffer.
    ///
    /// # Arguments
    ///
    /// * `matches` - Search matches to highlight
    pub fn highlight_matches(&self, matches: &[SearchMatch]) {
        // Remove previous highlights
        self.remove_highlights();

        // Apply new highlights
        for match_result in matches {
            // Find the line in buffer
            if let Some(start_pos) = self.line_starts.borrow().get(match_result.line_number) {
                let line_start = self.buffer.iter_at_offset(*start_pos as i32);
                let match_start = self.buffer.iter_at_offset(
                    (*start_pos + match_result.start_offset) as i32
                );
                let match_end = self.buffer.iter_at_offset(
                    (*start_pos + match_result.end_offset) as i32
                );

                if let Some(tag) = self.tag_table.lookup("search-highlight") {
                    self.buffer.apply_tag(&tag, &match_start, &match_end);
                }
            }
        }
    }

    /// Remove all search highlights.
    pub fn remove_highlights(&self) {
        if let Some(tag) = self.tag_table.lookup("search-highlight") {
            let start = self.buffer.start_iter();
            let end = self.buffer.end_iter();
            self.buffer.remove_tag(&tag, &start, &end);
        }
    }

    /// Scroll to bottom (via mark).
    pub fn scroll_to_bottom(&self) {
        // Create or move scroll mark
        let end = self.buffer.end_iter();
        if let Some(mark) = self.buffer.get_mark("scroll-mark") {
            self.buffer.move_mark(&mark, &end);
        } else {
            self.buffer.create_mark(Some("scroll-mark"), &end, false);
        }
    }

    /// Get current line count.
    pub fn line_count(&self) -> usize {
        self.line_queue.borrow().len()
    }

    /// Get maximum line capacity.
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Get total evicted lines.
    pub fn evicted_count(&self) -> u64 {
        *self.evicted_count.borrow()
    }

    /// Get buffer content as string.
    pub fn content(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, true).to_string()
    }

    /// Get recent lines (last N lines).
    pub fn get_recent_lines(&self, count: usize) -> Vec<String> {
        let line_queue = self.line_queue.borrow();
        let start = line_queue.len().saturating_sub(count);
        line_queue.iter()
            .skip(start)
            .map(|m| m.content.clone())
            .collect()
    }

    /// Update style configuration.
    ///
    /// Recreates text tags with new colors.
    pub fn update_style(&self, new_style: TerminalStyle) {
        // Note: GTK4 TextTagTable doesn't allow removing tags,
        // so we add new tags with updated names
        // In practice, this would require recreating the buffer
        // This is a simplified implementation
    }

    /// Get the underlying GTK4 TextBuffer.
    pub fn gtk_buffer(&self) -> &TextBuffer {
        &self.buffer
    }
}

impl Default for TerminalBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.max_lines(), DEFAULT_MAX_LINES);
        assert_eq!(buffer.line_count(), 0);
    }

    #[test]
    fn test_append_text() {
        let buffer = TerminalBuffer::new();
        buffer.append("Hello World\n", None);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.get_line(0), Some("Hello World".to_string()));
    }

    #[test]
    fn test_append_with_style() {
        let buffer = TerminalBuffer::new();
        buffer.append("Error message\n", Some(TextStyle::Error));
        assert_eq!(buffer.line_count(), 1);
    }

    #[test]
    fn test_append_multiple_lines() {
        let buffer = TerminalBuffer::new();
        buffer.append("Line 1\nLine 2\nLine 3\n", None);
        assert_eq!(buffer.line_count(), 3);
    }

    #[test]
    fn test_clear_buffer() {
        let buffer = TerminalBuffer::new();
        buffer.append("Test content\n", None);
        assert_eq!(buffer.line_count(), 1);

        buffer.clear();
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.evicted_count(), 0);
    }

    #[test]
    fn test_search_plain_text() {
        let buffer = TerminalBuffer::new();
        buffer.append("Hello World\nError: timeout\nSuccess: OK\n", None);

        let matches = buffer.search("Error", false);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line_number, 1);
        assert!(matches[0].matched_text.contains("Error"));
    }

    #[test]
    fn test_search_regex() {
        let buffer = TerminalBuffer::new();
        buffer.append("2024-01-15 Error\n2024-01-16 Warning\n", None);

        // Search for date pattern
        let matches = buffer.search("2024-[0-9]+-[0-9]+", true);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_get_recent_lines() {
        let buffer = TerminalBuffer::new();
        for i in 0..20 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        let recent = buffer.get_recent_lines(5);
        assert_eq!(recent.len(), 5);
        assert!(recent[0].contains("Line 15"));
        assert!(recent[4].contains("Line 19"));
    }

    #[test]
    fn test_fifo_eviction() {
        let buffer = TerminalBuffer::with_max_lines(10);

        for i in 0..15 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        // Should have only 10 lines (evicted 5)
        assert_eq!(buffer.line_count(), 10);
        assert_eq!(buffer.evicted_count(), 5);

        // Oldest line should be "Line 5"
        assert_eq!(buffer.get_line(0), Some("Line 5".to_string()));
    }

    #[test]
    fn test_ansi_sequence_parsing() {
        let buffer = TerminalBuffer::new();
        buffer.append_with_ansi("\x1b[31mRed text\x1b[0m Normal text\n");

        assert_eq!(buffer.line_count(), 1);
        // Content should have ANSI codes processed
        let content = buffer.content();
        assert!(content.contains("Red text"));
    }

    #[test]
    fn test_text_style_to_tag() {
        let buffer = TerminalBuffer::new();

        assert_eq!(buffer.style_to_tag_name(TextStyle::Bold), "bold");
        assert_eq!(buffer.style_to_tag_name(TextStyle::ForegroundColor(5)), "fg-5");
        assert_eq!(buffer.style_to_tag_name(TextStyle::Error), "error");
    }

    #[test]
    fn test_search_match_creation() {
        let match_result = SearchMatch::new(0, 5, 10, "test", "This is a test line");
        assert_eq!(match_result.line_number, 0);
        assert_eq!(match_result.start_offset, 5);
        assert_eq!(match_result.end_offset, 10);
        assert_eq!(match_result.matched_text, "test");
    }
}