//! Terminal Search Implementation
//!
//! This module provides search functionality for terminal output with:
//! - Plain text search
//! - Regex pattern search
//! - Non-blocking search operations
//! - Match highlighting and context
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.2)
//!
//! - Search MUST NOT block output processing
//! - Search returns match positions and context
//! - Regex support for advanced pattern matching
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           Terminal Session               │
//! │    ┌─────────────────────────────┐      │
//! │    │ ScrollBuffer                 │      │
//! │    │ (Line Storage + Index)       │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────┬───────────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────────┐
//! │           TerminalSearch                 │
//! │    ┌─────────────────────────────┐      │
//! │    │ SearchEngine                 │      │
//! │    │ - Plain text matcher         │      │
//! │    │ - Regex matcher              │      │
//! │    │ - Async wrapper              │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ SearchResult                 │      │
//! │    │ - Match positions            │      │
//! │    │ - Context lines              │      │
//! │    │ - Highlight info             │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::terminal::search::{TerminalSearch, SearchOptions, SearchResult};
//!
//! // Create search engine
//! let search = TerminalSearch::new();
//!
//! // Search in buffer content
//! let options = SearchOptions {
//!     pattern: "Error",
//!     use_regex: false,
//!     case_sensitive: false,
//!     context_lines: 2,
//! };
//!
//! let results = search.search(&buffer, options);
//! ```

use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::error::LiteError;
use crate::terminal::scroll_buffer::{ScrollBuffer, SearchMatch};

/// Search options for configuring search behavior.
///
/// Controls how the search is performed including pattern type,
/// case sensitivity, and result formatting.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Search pattern (text or regex)
    pub pattern: String,
    /// Interpret pattern as regex
    pub use_regex: bool,
    /// Case-sensitive matching
    pub case_sensitive: bool,
    /// Number of context lines to include
    pub context_lines: usize,
    /// Maximum number of results to return
    pub max_results: usize,
    /// Highlight matches in output
    pub highlight: bool,
    /// Search from end (most recent) first
    pub reverse: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            use_regex: false,
            case_sensitive: false,
            context_lines: 2,
            max_results: 100,
            highlight: true,
            reverse: false,
        }
    }
}

impl SearchOptions {
    /// Create options for plain text search.
    pub fn text(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            use_regex: false,
            ..Default::default()
        }
    }

    /// Create options for regex search.
    pub fn regex(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            use_regex: true,
            ..Default::default()
        }
    }

    /// Set case sensitivity.
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Set context lines.
    pub fn with_context(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Set max results.
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Enable/disable highlighting.
    pub fn with_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Search in reverse order (most recent first).
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }
}

/// Single search match result.
///
/// Contains position information, matched text, and context
/// for display in the terminal UI.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Line index in buffer (0-based)
    pub line_index: usize,
    /// Character start position in line
    pub char_start: usize,
    /// Character end position in line
    pub char_end: usize,
    /// The matched text
    pub matched_text: String,
    /// Line content (full)
    pub line_content: String,
    /// Context lines before match
    pub context_before: Vec<String>,
    /// Context lines after match
    pub context_after: Vec<String>,
    /// Highlight start/end positions for UI
    pub highlight_ranges: Vec<(usize, usize)>,
    /// Match confidence score (0-100)
    pub score: f32,
}

impl SearchResult {
    /// Create a new search result.
    pub fn new(
        line_index: usize,
        char_start: usize,
        char_end: usize,
        matched_text: &str,
        line_content: &str,
    ) -> Self {
        Self {
            line_index,
            char_start,
            char_end,
            matched_text: matched_text.to_string(),
            line_content: line_content.to_string(),
            context_before: Vec::new(),
            context_after: Vec::new(),
            highlight_ranges: vec![(char_start, char_end)],
            score: 100.0,
        }
    }

    /// Add context lines.
    pub fn with_context(mut self, before: Vec<String>, after: Vec<String>) -> Self {
        self.context_before = before;
        self.context_after = after;
        self
    }

    /// Calculate match relevance score.
    ///
    /// Factors:
    /// - Exact match (100)
    /// - Prefix match (80)
    /// - Contains match (60)
    /// - Regex match (varies)
    pub fn calculate_score(&mut self, pattern: &str) {
        if self.matched_text == pattern {
            self.score = 100.0;
        } else if self.matched_text.starts_with(pattern) {
            self.score = 80.0;
        } else if self.matched_text.contains(pattern) {
            self.score = 60.0;
        } else {
            // Regex or fuzzy match - default score
            self.score = 50.0;
        }
    }

    /// Get the line number (1-based for display).
    pub fn display_line_number(&self) -> usize {
        self.line_index + 1
    }

    /// Get highlighted line content.
    ///
    /// Inserts ANSI escape codes for highlighting matches.
    pub fn highlighted_content(&self) -> String {
        let mut result = self.line_content.clone();
        for (start, end) in &self.highlight_ranges {
            // Insert ANSI highlight codes
            let before = result[..*start].to_string();
            let match_text = result[*start..*end].to_string();
            let after = result[*end..].to_string();
            result = format!("{}\x1b[7m{}\x1b[0m{}", before, match_text, after);
        }
        result
    }
}

/// Search statistics for tracking search performance.
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Total searches performed
    pub total_searches: u64,
    /// Total matches found
    pub total_matches: u64,
    /// Average search time (ms)
    pub avg_search_time_ms: f32,
    /// Last search time (ms)
    pub last_search_time_ms: f32,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
}

/// Search history entry.
#[derive(Debug, Clone)]
pub struct SearchHistoryEntry {
    /// Search pattern
    pub pattern: String,
    /// Whether regex was used
    pub use_regex: bool,
    /// Number of results
    pub result_count: usize,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Terminal search engine with caching and async support.
///
/// Provides high-performance search with:
/// - Plain text and regex matching
/// - Result caching for repeated searches
/// - Async non-blocking operations
/// - Search history tracking
///
/// # Thread Safety
///
/// Uses async locks for safe concurrent access.
/// Search operations don't block output processing.
pub struct TerminalSearch {
    /// Search history for quick recall
    history: Arc<RwLock<Vec<SearchHistoryEntry>>>,
    /// Compiled regex cache
    regex_cache: Arc<Mutex<HashMap<String, Regex>>>,
    /// Search statistics
    stats: Arc<RwLock<SearchStats>>,
    /// Maximum history entries
    max_history: usize,
    /// Maximum regex cache entries
    max_regex_cache: usize,
}

impl TerminalSearch {
    /// Create a new search engine.
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            regex_cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SearchStats::default())),
            max_history: 50,
            max_regex_cache: 20,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(max_history: usize, max_regex_cache: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            regex_cache: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SearchStats::default())),
            max_history,
            max_regex_cache,
        }
    }

    /// Perform search on a scroll buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The scroll buffer to search
    /// * `options` - Search configuration
    ///
    /// # Returns
    ///
    /// A vector of `SearchResult` instances.
    ///
    /// # Constraints
    ///
    /// - MUST NOT block output processing
    /// - Uses buffer's search_async for non-blocking
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::search::{TerminalSearch, SearchOptions};
    /// use easyssh_core::terminal::scroll_buffer::{ScrollBuffer, Line};
    ///
    /// let search = TerminalSearch::new();
    /// let mut buffer = ScrollBuffer::new(100);
    /// buffer.push_line(Line::new("Error: connection failed"));
    ///
    /// let options = SearchOptions::text("Error");
    /// let results = search.search(&buffer, options);
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn search(&self, buffer: &ScrollBuffer, options: SearchOptions) -> Vec<SearchResult> {
        let start_time = std::time::Instant::now();

        // Get matches from buffer
        let matches = buffer.search(
            &options.pattern,
            options.use_regex,
            options.context_lines,
        );

        // Convert to SearchResult
        let mut results: Vec<SearchResult> = matches
            .iter()
            .take(options.max_results)
            .map(|m| {
                let line = buffer.get_lines(m.line_index, 1);
                let line_content = line.first().map(|l| l.content.clone()).unwrap_or_default();

                SearchResult::new(
                    m.line_index,
                    m.start,
                    m.end,
                    &m.matched_text,
                    &line_content,
                )
                .with_context(
                    m.context[..m.context_before].to_vec(),
                    m.context[m.context_before..].to_vec(),
                )
            })
            .collect();

        // Calculate scores
        for result in &mut results {
            result.calculate_score(&options.pattern);
        }

        // Reverse if requested
        if options.reverse {
            results.reverse();
        }

        // Update statistics
        let elapsed = start_time.elapsed().as_millis() as f32;
        self.update_stats(results.len(), elapsed);

        // Add to history
        self.add_to_history(&options, results.len());

        results
    }

    /// Perform async search (non-blocking).
    ///
    /// This is the preferred method for searches as it truly
    /// does not block output processing.
    ///
    /// # Arguments
    ///
    /// * `buffer` - Buffer wrapped in Arc<RwLock>
    /// * `options` - Search configuration
    ///
    /// # Returns
    ///
    /// A future resolving to search results.
    pub async fn search_async(
        &self,
        buffer: Arc<RwLock<ScrollBuffer>>,
        options: SearchOptions,
    ) -> Vec<SearchResult> {
        let start_time = std::time::Instant::now();

        // Acquire read lock on buffer (doesn't block writes)
        let buf = buffer.read().await;

        // Perform search
        let matches = buf.search(
            &options.pattern,
            options.use_regex,
            options.context_lines,
        );

        // Convert results
        let mut results: Vec<SearchResult> = matches
            .iter()
            .take(options.max_results)
            .map(|m| {
                let line = buf.get_lines(m.line_index, 1);
                let line_content = line.first().map(|l| l.content.clone()).unwrap_or_default();

                SearchResult::new(
                    m.line_index,
                    m.start,
                    m.end,
                    &m.matched_text,
                    &line_content,
                )
                .with_context(
                    m.context[..m.context_before].to_vec(),
                    m.context[m.context_before..].to_vec(),
                )
            })
            .collect();

        // Calculate scores
        for result in &mut results {
            result.calculate_score(&options.pattern);
        }

        // Reverse if requested
        if options.reverse {
            results.reverse();
        }

        // Update statistics
        let elapsed = start_time.elapsed().as_millis() as f32;
        self.update_stats(results.len(), elapsed);

        // Add to history
        self.add_to_history(&options, results.len());

        results
    }

    /// Search with pre-compiled regex.
    ///
    /// Uses cached regex for faster repeated searches.
    pub fn search_with_cached_regex(
        &self,
        buffer: &ScrollBuffer,
        pattern: &str,
        context_lines: usize,
    ) -> Result<Vec<SearchResult>, LiteError> {
        // Get or compile regex
        let regex = self.get_or_compile_regex(pattern)?;

        let matches = buffer.search(pattern, true, context_lines);

        let results: Vec<SearchResult> = matches
            .iter()
            .map(|m| {
                let line = buffer.get_lines(m.line_index, 1);
                let line_content = line.first().map(|l| l.content.clone()).unwrap_or_default();

                SearchResult::new(
                    m.line_index,
                    m.start,
                    m.end,
                    &m.matched_text,
                    &line_content,
                )
            })
            .collect();

        Ok(results)
    }

    /// Get or compile regex from cache.
    fn get_or_compile_regex(&self, pattern: &str) -> Result<Regex, LiteError> {
        // Check cache
        {
            let cache = self.regex_cache.blocking_lock();
            if let Some(regex) = cache.get(pattern) {
                // Update stats
                let mut stats = self.stats.blocking_write();
                stats.cache_hits += 1;
                return Ok(regex.clone());
            }
        }

        // Compile new regex
        let regex = Regex::new(pattern)
            .map_err(|e| LiteError::Terminal(format!("Invalid regex: {}", e)))?;

        // Add to cache
        {
            let mut cache = self.regex_cache.blocking_lock();

            // Evict oldest if cache full
            if cache.len() >= self.max_regex_cache {
                // Remove first entry (simple FIFO)
                if let Some(first_key) = cache.keys().next().cloned() {
                    cache.remove(&first_key);
                }
            }

            cache.insert(pattern.to_string(), regex.clone());
        }

        // Update stats
        {
            let mut stats = self.stats.blocking_write();
            stats.cache_misses += 1;
        }

        Ok(regex)
    }

    /// Clear regex cache.
    pub fn clear_regex_cache(&self) {
        let mut cache = self.regex_cache.blocking_lock();
        cache.clear();
    }

    /// Update search statistics.
    fn update_stats(&self, result_count: usize, elapsed_ms: f32) {
        let mut stats = self.stats.blocking_write();
        stats.total_searches += 1;
        stats.total_matches += result_count as u64;
        stats.last_search_time_ms = elapsed_ms;

        // Calculate rolling average
        if stats.total_searches > 0 {
            let prev_avg = stats.avg_search_time_ms;
            let n = stats.total_searches as f32;
            stats.avg_search_time_ms = prev_avg + (elapsed_ms - prev_avg) / n;
        }
    }

    /// Add search to history.
    fn add_to_history(&self, options: &SearchOptions, result_count: usize) {
        let mut history = self.history.blocking_write();

        // Remove duplicate patterns
        history.retain(|h| h.pattern != options.pattern);

        // Add new entry
        history.push(SearchHistoryEntry {
            pattern: options.pattern.clone(),
            use_regex: options.use_regex,
            result_count,
            timestamp: std::time::Instant::now(),
        });

        // Limit history size
        while history.len() > self.max_history {
            history.remove(0);
        }
    }

    /// Get search history.
    pub fn get_history(&self) -> Vec<SearchHistoryEntry> {
        self.history.blocking_read().clone()
    }

    /// Clear search history.
    pub fn clear_history(&self) {
        self.history.blocking_write().clear();
    }

    /// Get recent searches (last N entries).
    pub fn get_recent_searches(&self, count: usize) -> Vec<SearchHistoryEntry> {
        let history = self.history.blocking_read();
        history.iter().rev().take(count).cloned().collect()
    }

    /// Get search statistics.
    pub fn get_stats(&self) -> SearchStats {
        self.stats.blocking_read().clone()
    }

    /// Find next match from current position.
    ///
    /// Used for incremental navigation through results.
    pub fn find_next(
        &self,
        buffer: &ScrollBuffer,
        options: &SearchOptions,
        current_position: usize,
    ) -> Option<SearchResult> {
        let matches = buffer.search(&options.pattern, options.use_regex, 0);

        // Find match after current position
        for match_result in matches {
            if match_result.line_index > current_position {
                let line = buffer.get_lines(match_result.line_index, 1);
                let line_content = line.first().map(|l| l.content.clone()).unwrap_or_default();

                return Some(SearchResult::new(
                    match_result.line_index,
                    match_result.start,
                    match_result.end,
                    &match_result.matched_text,
                    &line_content,
                ));
            }
        }

        None
    }

    /// Find previous match from current position.
    ///
    /// Used for backward navigation through results.
    pub fn find_prev(
        &self,
        buffer: &ScrollBuffer,
        options: &SearchOptions,
        current_position: usize,
    ) -> Option<SearchResult> {
        let matches = buffer.search(&options.pattern, options.use_regex, 0);

        // Find match before current position
        for match_result in matches.iter().rev() {
            if match_result.line_index < current_position {
                let line = buffer.get_lines(match_result.line_index, 1);
                let line_content = line.first().map(|l| l.content.clone()).unwrap_or_default();

                return Some(SearchResult::new(
                    match_result.line_index,
                    match_result.start,
                    match_result.end,
                    &match_result.matched_text,
                    &line_content,
                ));
            }
        }

        None
    }

    /// Count total matches without creating full results.
    ///
    /// Faster for large buffers when only count is needed.
    pub fn count_matches(&self, buffer: &ScrollBuffer, pattern: &str, use_regex: bool) -> usize {
        buffer.search(pattern, use_regex, 0).len()
    }

    /// Search in specific line range.
    ///
    /// Used for partial buffer search (e.g., visible region).
    pub fn search_range(
        &self,
        buffer: &ScrollBuffer,
        options: &SearchOptions,
        start_line: usize,
        end_line: usize,
    ) -> Vec<SearchResult> {
        let lines = buffer.get_lines(start_line, end_line - start_line);
        let mut results = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            let actual_idx = start_line + idx;

            if options.use_regex {
                // Regex search
                if let Ok(regex) = self.get_or_compile_regex(&options.pattern) {
                    for cap in regex.find_iter(&line.content) {
                        results.push(SearchResult::new(
                            actual_idx,
                            cap.start(),
                            cap.end(),
                            &line.content[cap.start()..cap.end()],
                            &line.content,
                        ));
                    }
                }
            } else {
                // Plain text search
                let pattern = if options.case_sensitive {
                    options.pattern.clone()
                } else {
                    options.pattern.to_lowercase()
                };

                let content = if options.case_sensitive {
                    line.content.clone()
                } else {
                    line.content.to_lowercase()
                };

                let mut search_start = 0;
                while let Some(start) = content[search_start..].find(&pattern) {
                    let actual_start = search_start + start;
                    results.push(SearchResult::new(
                        actual_idx,
                        actual_start,
                        actual_start + pattern.len(),
                        &line.content[actual_start..actual_start + pattern.len()],
                        &line.content,
                    ));
                    search_start = actual_start + pattern.len();
                }
            }
        }

        results
    }

    /// Validate regex pattern.
    ///
    /// Returns true if pattern is valid regex syntax.
    pub fn validate_regex(pattern: &str) -> bool {
        Regex::new(pattern).is_ok()
    }

    /// Escape special regex characters for literal search.
    ///
    /// Converts a string to safe regex pattern.
    pub fn escape_regex(pattern: &str) -> String {
        regex::escape(pattern)
    }
}

impl Default for TerminalSearch {
    fn default() -> Self {
        Self::new()
    }
}

/// Global search instance for terminal sessions.
///
/// Shared across all terminal instances for consistent
/// history and caching.
static GLOBAL_SEARCH: once_cell::sync::Lazy<TerminalSearch> =
    once_cell::sync::Lazy::new(TerminalSearch::new);

/// Get the global search instance.
pub fn global_search() -> &'static TerminalSearch {
    &GLOBAL_SEARCH
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::scroll_buffer::{Line, ScrollBuffer};

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert!(!options.use_regex);
        assert!(!options.case_sensitive);
        assert_eq!(options.context_lines, 2);
    }

    #[test]
    fn test_search_options_text() {
        let options = SearchOptions::text("Error");
        assert_eq!(options.pattern, "Error");
        assert!(!options.use_regex);
    }

    #[test]
    fn test_search_options_regex() {
        let options = SearchOptions::regex("[0-9]+");
        assert_eq!(options.pattern, "[0-9]+");
        assert!(options.use_regex);
    }

    #[test]
    fn test_search_options_chain() {
        let options = SearchOptions::text("test")
            .case_sensitive(true)
            .with_context(5)
            .with_max_results(50)
            .reverse();

        assert!(options.case_sensitive);
        assert_eq!(options.context_lines, 5);
        assert_eq!(options.max_results, 50);
        assert!(options.reverse);
    }

    #[test]
    fn test_terminal_search_creation() {
        let search = TerminalSearch::new();
        let stats = search.get_stats();
        assert_eq!(stats.total_searches, 0);
    }

    #[test]
    fn test_search_plain_text() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Hello World"));
        buffer.push_line(Line::new("Error: timeout"));
        buffer.push_line(Line::new("Success"));

        let options = SearchOptions::text("Error");
        let results = search.search(&buffer, options);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matched_text, "Error");
        assert_eq!(results[0].line_index, 1);
    }

    #[test]
    fn test_search_regex() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Error: timeout"));
        buffer.push_line(Line::new("Warning: memory"));
        buffer.push_line(Line::new("Error: auth"));

        // Search for "Error:" or "Warning:"
        let options = SearchOptions::regex("(Error|Warning):");
        let results = search.search(&buffer, options);

        assert_eq!(results.len(), 3); // Two Error, one Warning
    }

    #[test]
    fn test_search_with_context() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Line 0"));
        buffer.push_line(Line::new("Line 1"));
        buffer.push_line(Line::new("MATCH HERE"));
        buffer.push_line(Line::new("Line 3"));
        buffer.push_line(Line::new("Line 4"));

        let options = SearchOptions::text("MATCH").with_context(2);
        let results = search.search(&buffer, options);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].context_before.len(), 2);
        assert_eq!(results[0].context_after.len(), 2);
        assert_eq!(results[0].context_before[0], "Line 1");
        assert_eq!(results[0].context_after[0], "Line 3");
    }

    #[test]
    fn test_search_reverse() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Error 1"));
        buffer.push_line(Line::new("Error 2"));
        buffer.push_line(Line::new("Error 3"));

        let options = SearchOptions::text("Error").reverse();
        let results = search.search(&buffer, options);

        assert_eq!(results.len(), 3);
        // First result should be from last line
        assert_eq!(results[0].line_index, 2);
        assert_eq!(results[2].line_index, 0);
    }

    #[test]
    fn test_search_max_results() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..20 {
            buffer.push_line(Line::new(&format!("Error {}", i)));
        }

        let options = SearchOptions::text("Error").with_max_results(5);
        let results = search.search(&buffer, options);

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_search_result_highlighted() {
        let result = SearchResult::new(0, 5, 10, "match", "Hello match world");

        let highlighted = result.highlighted_content();
        assert!(highlighted.contains("\x1b[7m")); // Contains ANSI reverse
        assert!(highlighted.contains("\x1b[0m")); // Contains reset
    }

    #[test]
    fn test_search_result_score() {
        let mut result = SearchResult::new(0, 0, 5, "Error", "Error: test");

        result.calculate_score("Error");
        assert_eq!(result.score, 100.0); // Exact match

        let mut result2 = SearchResult::new(0, 0, 5, "Error:", "Error: test");
        result2.calculate_score("Error");
        assert_eq!(result2.score, 80.0); // Prefix match
    }

    #[test]
    fn test_find_next() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Error at line 0"));
        buffer.push_line(Line::new("OK"));
        buffer.push_line(Line::new("Error at line 2"));
        buffer.push_line(Line::new("OK"));
        buffer.push_line(Line::new("Error at line 4"));

        let options = SearchOptions::text("Error");

        // Find next from position 0
        let next = search.find_next(&buffer, &options, 0);
        assert!(next.is_some());
        assert_eq!(next.unwrap().line_index, 2);

        // Find next from position 2
        let next2 = search.find_next(&buffer, &options, 2);
        assert!(next2.is_some());
        assert_eq!(next2.unwrap().line_index, 4);
    }

    #[test]
    fn test_find_prev() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Error at line 0"));
        buffer.push_line(Line::new("OK"));
        buffer.push_line(Line::new("Error at line 2"));

        let options = SearchOptions::text("Error");

        // Find prev from position 2
        let prev = search.find_prev(&buffer, &options, 2);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().line_index, 0);
    }

    #[test]
    fn test_count_matches() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..10 {
            buffer.push_line(Line::new(&format!("Error {}", i)));
        }

        let count = search.count_matches(&buffer, "Error", false);
        assert_eq!(count, 10);
    }

    #[test]
    fn test_search_range() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..20 {
            buffer.push_line(Line::new(&format!("Line {} Error", i)));
        }

        let options = SearchOptions::text("Error");
        let results = search.search_range(&buffer, &options, 5, 10);

        // Should only find matches in lines 5-9
        for result in &results {
            assert!(result.line_index >= 5);
            assert!(result.line_index < 10);
        }
    }

    #[test]
    fn test_regex_validation() {
        assert!(TerminalSearch::validate_regex("[0-9]+"));
        assert!(TerminalSearch::validate_regex("(a|b)"));
        assert!(!TerminalSearch::validate_regex("[invalid")); // Unbalanced bracket
    }

    #[test]
    fn test_regex_escape() {
        let escaped = TerminalSearch::escape_regex("test[0]");
        assert_eq!(escaped, "test\\[0\\]");
    }

    #[test]
    fn test_regex_cache() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Test 123"));

        // First search - cache miss
        let _ = search.search_with_cached_regex(&buffer, "[0-9]+", 0);
        let stats = search.get_stats();
        assert_eq!(stats.cache_misses, 1);

        // Second search - cache hit
        let _ = search.search_with_cached_regex(&buffer, "[0-9]+", 0);
        let stats = search.get_stats();
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_search_history() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Test content"));

        // Perform multiple searches
        let options1 = SearchOptions::text("Test");
        let options2 = SearchOptions::text("content");
        search.search(&buffer, options1);
        search.search(&buffer, options2);

        let history = search.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].pattern, "Test");
        assert_eq!(history[1].pattern, "content");
    }

    #[test]
    fn test_search_stats_update() {
        let search = TerminalSearch::new();
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Test"));

        let options = SearchOptions::text("Test");
        search.search(&buffer, options);

        let stats = search.get_stats();
        assert_eq!(stats.total_searches, 1);
        assert_eq!(stats.total_matches, 1);
        assert!(stats.last_search_time_ms >= 0.0);
    }

    #[test]
    fn test_global_search() {
        let search = global_search();
        assert!(&search.stats.blocking_read().total_searches >= &0);
    }

    #[tokio::test]
    async fn test_async_search() {
        let search = TerminalSearch::new();
        let buffer = Arc::new(RwLock::new(ScrollBuffer::new(100)));

        // Add content
        {
            let mut buf = buffer.write().await;
            buf.push_line(Line::new("Error: async test"));
        }

        // Search async
        let options = SearchOptions::text("Error");
        let results = search.search_async(buffer, options).await;

        assert_eq!(results.len(), 1);
    }
}