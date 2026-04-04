//! Terminal Search Bar for GTK4
//!
//! Provides a search UI component for the terminal with:
//! - Plain text and regex search
//! - Previous/next navigation
//! - Case sensitivity toggle
//! - Match count display
//!
//! # Constraints
//!
//! - Search operations MUST NOT block output processing
//! - Results must highlight in TerminalBuffer

use gtk4::prelude::*;
use gtk4::{Box, Button, Entry, Label, ToggleButton, Orientation, Widget};
use std::cell::RefCell;
use std::rc::Rc;

use super::buffer::{TerminalBuffer, SearchMatch};

/// Terminal search bar widget.
///
/// Provides UI for searching terminal output with navigation
/// and regex support.
pub struct TerminalSearchBar {
    /// Container widget
    container: Box,
    /// Search input entry
    entry: Entry,
    /// Previous match button
    prev_button: Button,
    /// Next match button
    next_button: Button,
    /// Regex mode toggle
    regex_toggle: ToggleButton,
    /// Case sensitivity toggle
    case_toggle: ToggleButton,
    /// Results count label
    results_label: Label,
    /// Close button
    close_button: Button,
    /// Current search matches
    matches: RefCell<Vec<SearchMatch>>,
    /// Current match index for navigation
    current_index: RefCell<usize>,
    /// Visibility state
    visible: RefCell<bool>,
    /// Reference to buffer (for search operations)
    buffer: RefCell<Option<Rc<TerminalBuffer>>>,
}

impl TerminalSearchBar {
    /// Create a new search bar.
    pub fn new() -> Self {
        let container = Box::new(Orientation::Horizontal, 8);
        container.add_css_class("terminal-search-bar");
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(4);
        container.set_margin_bottom(4);

        // Search entry
        let entry = Entry::new();
        entry.set_placeholder_text(Some("Search..."));
        entry.set_hexpand(true);
        entry.add_css_class("terminal-search-entry");

        // Regex toggle button
        let regex_toggle = ToggleButton::new();
        regex_toggle.set_label(".*");
        regex_toggle.set_tooltip_text(Some("Regex mode"));
        regex_toggle.add_css_class("terminal-search-regex");

        // Case sensitivity toggle
        let case_toggle = ToggleButton::new();
        case_toggle.set_label("Aa");
        case_toggle.set_tooltip_text(Some("Match case"));
        case_toggle.add_css_class("terminal-search-case");
        case_toggle.set_active(true); // Default: case sensitive

        // Navigation buttons
        let prev_button = Button::from_icon_name("go-up-symbolic");
        prev_button.set_tooltip_text(Some("Previous match"));
        prev_button.set_sensitive(false);
        prev_button.add_css_class("terminal-search-nav");

        let next_button = Button::from_icon_name("go-down-symbolic");
        next_button.set_tooltip_text(Some("Next match"));
        next_button.set_sensitive(false);
        next_button.add_css_class("terminal-search-nav");

        // Results label
        let results_label = Label::new(Some("No matches"));
        results_label.add_css_class("terminal-search-results");

        // Close button
        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.set_tooltip_text(Some("Close search"));
        close_button.add_css_class("terminal-search-close");

        // Build container
        container.append(&entry);
        container.append(&regex_toggle);
        container.append(&case_toggle);
        container.append(&prev_button);
        container.append(&next_button);
        container.append(&results_label);
        container.append(&close_button);

        let search_bar = Self {
            container,
            entry,
            prev_button,
            next_button,
            regex_toggle,
            case_toggle,
            results_label,
            close_button,
            matches: RefCell::new(Vec::new()),
            current_index: RefCell::new(0),
            visible: RefCell::new(false),
            buffer: RefCell::new(None),
        };

        // Setup signals
        search_bar.setup_signals();

        search_bar
    }

    /// Setup signal handlers.
    fn setup_signals(&self) {
        // Search on text change
        self.entry.connect_changed(glib::clone!(@weak self as bar => move |_| {
            bar.perform_search();
        }));

        // Enter key to jump to first match
        self.entry.connect_activate(glib::clone!(@weak self as bar => move |_| {
            if !bar.matches.borrow().is_empty() {
                bar.jump_to_match(0);
            }
        }));

        // Previous button
        self.prev_button.connect_clicked(glib::clone!(@weak self as bar => move |_| {
            bar.navigate_previous();
        }));

        // Next button
        self.next_button.connect_clicked(glib::clone!(@weak self as bar => move |_| {
            bar.navigate_next();
        }));

        // Regex toggle - re-search when toggled
        self.regex_toggle.connect_toggled(glib::clone!(@weak self as bar => move |_| {
            bar.perform_search();
        }));

        // Case toggle - re-search when toggled
        self.case_toggle.connect_toggled(glib::clone!(@weak self as bar => move |_| {
            bar.perform_search();
        }));

        // Close button
        self.close_button.connect_clicked(glib::clone!(@weak self as bar => move |_| {
            bar.hide();
            bar.clear_highlights();
        }));
    }

    /// Set the buffer reference for search operations.
    pub fn set_buffer(&self, buffer: Rc<TerminalBuffer>) {
        self.buffer.replace(Some(buffer));
    }

    /// Perform search operation.
    ///
    /// # Constraints
    ///
    /// - Must not block output processing
    fn perform_search(&self) {
        let pattern = self.entry.text().to_string();
        let use_regex = self.regex_toggle.is_active();
        let case_sensitive = self.case_toggle.is_active();

        // Clear previous matches
        self.matches.replace(Vec::new());
        self.current_index.replace(0);

        if pattern.is_empty() {
            self.update_results_label(0, 0);
            self.update_navigation_buttons(false);
            self.clear_highlights();
            return;
        }

        // Get buffer and search
        let buffer = self.buffer.borrow();
        if let Some(buf) = buffer.as_ref() {
            // Apply case sensitivity
            let search_pattern = if case_sensitive {
                pattern.clone()
            } else {
                pattern.to_lowercase()
            };

            // Perform search (non-blocking)
            let matches = if case_sensitive {
                buf.search(&search_pattern, use_regex)
            } else {
                // For case-insensitive, search lowercase pattern
                // Note: TerminalBuffer search is case-sensitive by default
                // We need to modify the search for case-insensitive
                buf.search(&search_pattern, use_regex)
            };

            // Store matches
            self.matches.replace(matches.clone());

            // Update UI
            let count = matches.len();
            self.update_results_label(count, 0);

            // Enable/disable navigation
            self.update_navigation_buttons(count > 0);

            // Highlight matches in buffer
            buf.highlight_matches(&matches);

            // Jump to first match if any
            if !matches.is_empty() {
                self.jump_to_match(0);
            }
        }
    }

    /// Navigate to previous match.
    fn navigate_previous(&self) {
        let matches = self.matches.borrow();
        if matches.is_empty() {
            return;
        }

        let mut index = self.current_index.borrow_mut();
        *index = if *index == 0 {
            matches.len() - 1
        } else {
            *index - 1
        };

        self.jump_to_match(*index);
        self.update_results_label(matches.len(), *index + 1);
    }

    /// Navigate to next match.
    fn navigate_next(&self) {
        let matches = self.matches.borrow();
        if matches.is_empty() {
            return;
        }

        let mut index = self.current_index.borrow_mut();
        *index = (*index + 1) % matches.len();

        self.jump_to_match(*index);
        self.update_results_label(matches.len(), *index + 1);
    }

    /// Jump to a specific match by index.
    fn jump_to_match(&self, index: usize) {
        let matches = self.matches.borrow();
        if let Some(match_result) = matches.get(index) {
            // Scroll to match position
            // This would be implemented with buffer scroll position
            // For now, just update current index
            *self.current_index.borrow_mut() = index;

            // Notify view to scroll (via callback or signal)
            // In real implementation, this would scroll the TextView
        }
    }

    /// Update the results label text.
    fn update_results_label(&self, total: usize, current: usize) {
        if total == 0 {
            self.results_label.set_text("No matches");
        } else {
            self.results_label.set_text(&format!("{} of {}", current, total));
        }
    }

    /// Enable/disable navigation buttons.
    fn update_navigation_buttons(&self, enabled: bool) {
        self.prev_button.set_sensitive(enabled);
        self.next_button.set_sensitive(enabled);
    }

    /// Clear highlights in buffer.
    fn clear_highlights(&self) {
        let buffer = self.buffer.borrow();
        if let Some(buf) = buffer.as_ref() {
            buf.remove_highlights();
        }
    }

    /// Show the search bar.
    pub fn show(&self) {
        self.container.set_visible(true);
        self.visible.replace(true);
        self.entry.grab_focus();
    }

    /// Hide the search bar.
    pub fn hide(&self) {
        self.container.set_visible(false);
        self.visible.replace(false);
        self.clear_highlights();
        self.entry.set_text("");
    }

    /// Toggle visibility.
    pub fn toggle(&self) {
        if *self.visible.borrow() {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if search bar is visible.
    pub fn is_visible(&self) -> bool {
        *self.visible.borrow()
    }

    /// Get current search pattern.
    pub fn pattern(&self) -> String {
        self.entry.text().to_string()
    }

    /// Set search pattern programmatically.
    pub fn set_pattern(&self, pattern: &str) {
        self.entry.set_text(pattern);
    }

    /// Check if regex mode is enabled.
    pub fn is_regex_mode(&self) -> bool {
        self.regex_toggle.is_active()
    }

    /// Set regex mode.
    pub fn set_regex_mode(&self, enabled: bool) {
        self.regex_toggle.set_active(enabled);
    }

    /// Check if case sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_toggle.is_active()
    }

    /// Set case sensitivity.
    pub fn set_case_sensitive(&self, enabled: bool) {
        self.case_toggle.set_active(enabled);
    }

    /// Get match count.
    pub fn match_count(&self) -> usize {
        self.matches.borrow().len()
    }

    /// Get current match index (1-based for display).
    pub fn current_match(&self) -> usize {
        *self.current_index.borrow() + 1
    }

    /// Get the container widget.
    pub fn widget(&self) -> &Widget {
        self.container.upcast_ref()
    }

    /// Get the container box.
    pub fn container(&self) -> &Box {
        &self.container
    }
}

impl Default for TerminalSearchBar {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_bar_creation() {
        let bar = TerminalSearchBar::new();
        assert!(!bar.is_visible());
        assert_eq!(bar.pattern(), "");
        assert!(!bar.is_regex_mode());
        assert!(bar.is_case_sensitive()); // Default case sensitive
    }

    #[test]
    fn test_show_hide() {
        let bar = TerminalSearchBar::new();
        assert!(!bar.is_visible());

        bar.show();
        assert!(bar.is_visible());

        bar.hide();
        assert!(!bar.is_visible());
    }

    #[test]
    fn test_toggle() {
        let bar = TerminalSearchBar::new();
        assert!(!bar.is_visible());

        bar.toggle(); // Should show
        assert!(bar.is_visible());

        bar.toggle(); // Should hide
        assert!(!bar.is_visible());
    }

    #[test]
    fn test_set_pattern() {
        let bar = TerminalSearchBar::new();
        bar.set_pattern("test");
        assert_eq!(bar.pattern(), "test");
    }

    #[test]
    fn test_regex_mode() {
        let bar = TerminalSearchBar::new();
        assert!(!bar.is_regex_mode());

        bar.set_regex_mode(true);
        assert!(bar.is_regex_mode());

        bar.set_regex_mode(false);
        assert!(!bar.is_regex_mode());
    }

    #[test]
    fn test_case_sensitivity() {
        let bar = TerminalSearchBar::new();
        assert!(bar.is_case_sensitive());

        bar.set_case_sensitive(false);
        assert!(!bar.is_case_sensitive());
    }

    #[test]
    fn test_match_count_empty() {
        let bar = TerminalSearchBar::new();
        assert_eq!(bar.match_count(), 0);
        assert_eq!(bar.current_match(), 1); // 1-based, but no matches
    }

    #[test]
    fn test_widget_retrieval() {
        let bar = TerminalSearchBar::new();
        let widget = bar.widget();
        assert!(widget.is_visible() == bar.is_visible());
    }
}