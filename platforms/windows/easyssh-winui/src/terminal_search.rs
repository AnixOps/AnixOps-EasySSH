#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, RichText, TextFormat, Ui};
use std::collections::VecDeque;

/// Terminal content search functionality
pub struct TerminalSearch {
    /// Current search query
    query: String,
    /// Search results (line numbers and positions)
    results: Vec<SearchMatch>,
    /// Current result index
    current_index: usize,
    /// Search options
    options: SearchOptions,
    /// Search history
    history: VecDeque<String>,
    /// Maximum history size
    max_history: usize,
    /// Whether search is case-sensitive
    case_sensitive: bool,
    /// Whether to use regex
    use_regex: bool,
    /// Whether to search backwards
    search_backwards: bool,
    /// Show search UI
    show_ui: bool,
    /// Last search term for detecting changes
    last_query: String,
    /// Total lines searched
    total_lines: usize,
    /// Search statistics
    search_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub matched_text: String,
    pub line_content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
        }
    }
}

/// Response from terminal search UI
#[derive(Debug, Default)]
pub struct TerminalSearchResponse {
    pub search_requested: bool,
    pub navigate_next: bool,
    pub navigate_prev: bool,
    pub close_requested: bool,
    pub query_changed: bool,
    pub current_match: Option<SearchMatch>,
}

impl TerminalSearch {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            current_index: 0,
            options: SearchOptions::default(),
            history: VecDeque::with_capacity(20),
            max_history: 20,
            case_sensitive: false,
            use_regex: false,
            search_backwards: false,
            show_ui: false,
            last_query: String::new(),
            total_lines: 0,
            search_time_ms: 0,
        }
    }

    pub fn with_options(options: SearchOptions) -> Self {
        let mut search = Self::new();
        search.options = options;
        search.case_sensitive = options.case_sensitive;
        search.use_regex = options.use_regex;
        search
    }

    /// Show/hide the search UI
    pub fn toggle(&mut self) {
        self.show_ui = !self.show_ui;
    }

    pub fn show(&mut self) {
        self.show_ui = true;
    }

    pub fn hide(&mut self) {
        self.show_ui = false;
    }

    pub fn is_visible(&self) -> bool {
        self.show_ui
    }

    /// Set the search query
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
    }

    /// Get the current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Search in terminal content
    pub fn search(&mut self, content: &str) -> Vec<SearchMatch> {
        let start_time = std::time::Instant::now();
        self.results.clear();
        self.current_index = 0;
        self.total_lines = 0;

        if self.query.is_empty() {
            self.search_time_ms = 0;
            return Vec::new();
        }

        let lines: Vec<&str> = content.lines().collect();
        self.total_lines = lines.len();

        for (line_num, line) in lines.iter().enumerate() {
            let matches = self.find_matches_in_line(line, line_num);
            self.results.extend(matches);
        }

        // Add to history if results found and query is new
        if !self.results.is_empty() && !self.history.contains(&self.query) {
            self.add_to_history(self.query.clone());
        }

        self.last_query = self.query.clone();
        self.search_time_ms = start_time.elapsed().as_millis() as u64;

        self.results.clone()
    }

    fn find_matches_in_line(&self, line: &str, line_num: usize) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        let search_str = if self.options.case_sensitive || self.case_sensitive {
            line.to_string()
        } else {
            line.to_lowercase()
        };
        let query = if self.options.case_sensitive || self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        if self.options.use_regex || self.use_regex {
            // Simple regex-like support (fallback to literal if invalid)
            if let Ok(regex) = regex::Regex::new(&self.query) {
                for mat in regex.find_iter(line) {
                    matches.push(SearchMatch {
                        line_number: line_num,
                        char_start: mat.start(),
                        char_end: mat.end(),
                        matched_text: mat.as_str().to_string(),
                        line_content: line.to_string(),
                    });
                }
            } else {
                // Fallback to literal search
                self.find_literal_matches(&search_str, &query, line, line_num, &mut matches);
            }
        } else {
            self.find_literal_matches(&search_str, &query, line, line_num, &mut matches);
        }

        matches
    }

    fn find_literal_matches(
        &self,
        search_str: &str,
        query: &str,
        original_line: &str,
        line_num: usize,
        matches: &mut Vec<SearchMatch>,
    ) {
        let mut start = 0;
        while let Some(pos) = search_str[start..].find(query) {
            let match_start = start + pos;
            let match_end = match_start + query.len();

            // Check whole word if needed
            if self.options.whole_word {
                let is_word_start = match_start == 0
                    || !original_line.chars().nth(match_start.saturating_sub(1)).unwrap_or(' ')
                        .is_alphanumeric();
                let is_word_end = match_end >= original_line.len()
                    || !original_line.chars().nth(match_end).unwrap_or(' ')
                        .is_alphanumeric();
                if !is_word_start || !is_word_end {
                    start = match_end;
                    continue;
                }
            }

            matches.push(SearchMatch {
                line_number: line_num,
                char_start: match_start,
                char_end: match_end,
                matched_text: original_line[match_start..match_end].to_string(),
                line_content: original_line.to_string(),
            });

            start = match_end;
        }
    }

    /// Navigate to next match
    pub fn next_match(&mut self) -> Option<&SearchMatch> {
        if self.results.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.results.len();
        self.results.get(self.current_index)
    }

    /// Navigate to previous match
    pub fn prev_match(&mut self) -> Option<&SearchMatch> {
        if self.results.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.results.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.results.get(self.current_index)
    }

    /// Get current match
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.results.get(self.current_index)
    }

    /// Get all results
    pub fn results(&self) -> &[SearchMatch] {
        &self.results
    }

    /// Get result count
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Clear search results
    pub fn clear(&mut self) {
        self.results.clear();
        self.current_index = 0;
        self.query.clear();
        self.last_query.clear();
    }

    fn add_to_history(&mut self, query: String) {
        if query.is_empty() {
            return;
        }
        // Remove if exists to avoid duplicates
        self.history.retain(|q| q != &query);
        self.history.push_front(query);
        if self.history.len() > self.max_history {
            self.history.pop_back();
        }
    }

    /// Render the search UI
    pub fn ui(&mut self, ui: &mut Ui, terminal_content: &str) -> TerminalSearchResponse {
        let mut response = TerminalSearchResponse::default();

        if !self.show_ui {
            return response;
        }

        // Check if query changed
        if self.query != self.last_query {
            response.query_changed = true;
        }

        ui.horizontal(|ui| {
            ui.label(egui::icons::ICON_SEARCH);

            // Search input
            let text_edit = egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search...")
                .desired_width(200.0);
            let input_response = ui.add(text_edit);

            // Options button
            ui.menu_button("⚙", |ui| {
                ui.checkbox(&mut self.case_sensitive, "Case sensitive");
                ui.checkbox(&mut self.use_regex, "Regex");
                ui.checkbox(&mut self.options.whole_word, "Whole word");
            });

            // Search navigation
            ui.add_space(8.0);
            ui.set_enabled(!self.results.is_empty());
            if ui.button("◀").clicked() {
                response.navigate_prev = true;
                if let Some(m) = self.prev_match() {
                    response.current_match = Some(m.clone());
                }
            }
            if ui.button("▶").clicked() {
                response.navigate_next = true;
                if let Some(m) = self.next_match() {
                    response.current_match = Some(m.clone());
                }
            }
            ui.set_enabled(true);

            // Result counter
            if !self.results.is_empty() {
                ui.label(
                    RichText::new(format!("{}/{}", self.current_index + 1, self.results.len()))
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
            }

            // Close button
            if ui.button("✕").clicked() {
                self.hide();
                response.close_requested = true;
            }

            // Handle Enter key to search
            if input_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.query.is_empty()
            {
                response.search_requested = true;
                self.search(terminal_content);
            }

            // Auto-search on query change
            if response.query_changed && !self.query.is_empty() && self.query.len() > 2 {
                self.search(terminal_content);
            }
        });

        // Search statistics
        if !self.results.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Found {} matches in {} lines ({:.1}ms)",
                        self.results.len(),
                        self.total_lines,
                        self.search_time_ms as f64
                    ))
                    .size(11.0)
                    .color(Color32::from_gray(150)),
                );
            });
        }

        // Search history dropdown
        if !self.history.is_empty() {
            ui.horizontal(|ui| {
                ui.label("History:");
                for past_query in self.history.iter().take(5) {
                    if ui.button(past_query).clicked() {
                        self.query = past_query.clone();
                        response.search_requested = true;
                        self.search(terminal_content);
                    }
                }
            });
        }

        response
    }

    /// Render search results in a popup
    pub fn render_results_popup(&self, ui: &mut Ui, scroll_to_match: bool) -> Option<usize> {
        let mut scroll_target: Option<usize> = None;

        if self.results.is_empty() || !self.show_ui {
            return None;
        }

        egui::Window::new("Search Results")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, result) in self.results.iter().enumerate() {
                        let is_current = idx == self.current_index;

                        let bg_color = if is_current {
                            Color32::from_rgb(60, 60, 80)
                        } else {
                            Color32::from_gray(40)
                        };

                        egui::Frame::group(ui.style())
                            .fill(bg_color)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("Line {}", result.line_number + 1))
                                            .strong()
                                            .color(Color32::YELLOW),
                                    );

                                    if is_current {
                                        ui.label(
                                            RichText::new("●").color(Color32::GREEN).size(12.0),
                                        );
                                    }
                                });

                                // Show line content with highlighted match
                                let content = &result.line_content;
                                let mut job = egui::text::LayoutJob::default();

                                // Add text before match
                                if result.char_start > 0 {
                                    job.append(
                                        &content[..result.char_start],
                                        0.0,
                                        TextFormat::default(),
                                    );
                                }

                                // Add matched text with highlight
                                job.append(
                                    &result.matched_text,
                                    0.0,
                                    TextFormat {
                                        background: Color32::YELLOW,
                                        color: Color32::BLACK,
                                        ..Default::default()
                                    },
                                );

                                // Add text after match
                                if result.char_end < content.len() {
                                    job.append(
                                        &content[result.char_end..],
                                        0.0,
                                        TextFormat::default(),
                                    );
                                }

                                ui.label(job);
                            });

                        if is_current && scroll_to_match {
                            scroll_target = Some(result.line_number);
                        }
                    }
                });
            });

        scroll_target
    }

    /// Highlight search matches in terminal text with egui formatting
    pub fn highlight_matches(&self, line: &str, line_number: usize) -> Option<egui::text::LayoutJob> {
        let line_matches: Vec<_> = self
            .results
            .iter()
            .filter(|m| m.line_number == line_number)
            .collect();

        if line_matches.is_empty() {
            return None;
        }

        let mut job = egui::text::LayoutJob::default();
        let mut last_end = 0;

        for result in line_matches {
            // Add text before match
            if result.char_start > last_end {
                job.append(&line[last_end..result.char_start], 0.0, TextFormat::default());
            }

            // Add matched text with highlight
            let is_current = self.results.get(self.current_index).map(|m| m as *const _)
                == Some(result as *const _);

            let highlight_color = if is_current {
                Color32::from_rgb(255, 200, 0) // Bright yellow for current
            } else {
                Color32::from_rgb(100, 80, 40) // Darker for others
            };

            job.append(
                &result.matched_text,
                0.0,
                TextFormat {
                    background: highlight_color,
                    color: Color32::WHITE,
                    ..Default::default()
                },
            );

            last_end = result.char_end;
        }

        // Add remaining text
        if last_end < line.len() {
            job.append(&line[last_end..], 0.0, TextFormat::default());
        }

        Some(job)
    }

    /// Keyboard shortcut handler
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) -> TerminalSearchResponse {
        let mut response = TerminalSearchResponse::default();

        // Ctrl+F to toggle search
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.toggle();
            response.search_requested = self.show_ui;
        }

        // F3 to find next
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            if !self.results.is_empty() {
                response.navigate_next = true;
                if let Some(m) = self.next_match() {
                    response.current_match = Some(m.clone());
                }
            }
        }

        // Shift+F3 to find previous
        if ctx.input(|i| i.modifiers.shift && i.key_pressed(egui::Key::F3)) {
            if !self.results.is_empty() {
                response.navigate_prev = true;
                if let Some(m) = self.prev_match() {
                    response.current_match = Some(m.clone());
                }
            }
        }

        // Escape to close search
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.show_ui {
            self.hide();
            response.close_requested = true;
        }

        response
    }
}

impl Default for TerminalSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_search() {
        let content = "Hello world\nHello universe\nGoodbye world";
        let mut search = TerminalSearch::new();
        search.set_query("world");
        let results = search.search(content);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 0);
        assert_eq!(results[1].line_number, 2);
    }

    #[test]
    fn test_case_insensitive_search() {
        let content = "Hello World\nHELLO world";
        let mut search = TerminalSearch::new();
        search.set_query("hello");
        let results = search.search(content);

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_navigation() {
        let content = "line1\nline2\nline3";
        let mut search = TerminalSearch::new();
        search.set_query("line");
        search.search(content);

        assert_eq!(search.result_count(), 3);

        let first = search.current_match().cloned();
        assert!(first.is_some());

        let second = search.next_match().cloned();
        assert_ne!(first, second);

        let third = search.next_match().cloned();
        assert_ne!(second, third);

        // Wrap around
        let back_to_first = search.next_match().cloned();
        assert_eq!(first, back_to_first);
    }
}
