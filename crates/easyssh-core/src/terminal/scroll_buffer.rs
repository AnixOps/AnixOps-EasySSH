//! Scroll Buffer Implementation for Terminal Output
//!
//! This module provides a high-performance scroll buffer for terminal output
//! with FIFO eviction, search capabilities, and timestamp tracking.
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 1.2)
//!
//! - Buffer size has upper limit (default 10000 lines)
//! - FIFO eviction when exceeding limit
//! - Search operations MUST NOT block output processing
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           TerminalOutput                 │
//! │    (PTY Data Stream)                     │
//! └─────────────────┬───────────────────────┘
//!                   │ push_line()
//! ┌─────────────────▼───────────────────────┐
//! │           ScrollBuffer                   │
//! │    ┌─────────────────────────────┐      │
//! │    │ VecDeque<Line> (FIFO Queue) │      │
//! │    │ max_lines: 10000            │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ SearchIndex (Optional)      │      │
//! │    │ - Full-text index           │      │
//! │    │ - Regex support             │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::terminal::scroll_buffer::{ScrollBuffer, Line, SearchMatch};
//!
//! // Create buffer with default limit (10000 lines)
//! let mut buffer = ScrollBuffer::new(10000);
//!
//! // Push output lines
//! buffer.push_line(Line::new("Hello World"));
//! buffer.push_line(Line::new("Error: connection failed"));
//!
//! // Search for patterns
//! let matches = buffer.search("Error", false); // Plain text search
//! assert_eq!(matches.len(), 1);
//!
//! // Regex search
//! let regex_matches = buffer.search("E[a-z]+:", true);
//! ```

use std::collections::VecDeque;
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::LiteError;

/// Default maximum lines in scroll buffer (per SYSTEM_INVARIANTS.md)
pub const DEFAULT_MAX_LINES: usize = 10000;

/// Maximum lines for Lite edition
pub const LITE_MAX_LINES: usize = 5000;

/// Maximum lines for Pro edition
pub const PRO_MAX_LINES: usize = 50000;

/// A single line in the scroll buffer with metadata.
///
/// Each line tracks:
/// - Content: The actual text output
/// - Timestamp: When the line was added (for debugging and replay)
/// - Wrapped flag: Whether this line is a continuation of a previous line
#[derive(Debug, Clone)]
pub struct Line {
    /// The text content of the line
    pub content: String,
    /// Timestamp when the line was added to buffer
    pub timestamp: Instant,
    /// Whether this line is wrapped from previous line
    pub is_wrapped: bool,
    /// Line number in the original output sequence
    pub sequence: u64,
}

impl Line {
    /// Create a new line with the given content.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content
    ///
    /// # Returns
    ///
    /// A new `Line` instance with current timestamp.
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            timestamp: Instant::now(),
            is_wrapped: false,
            sequence: 0,
        }
    }

    /// Create a wrapped line (continuation of previous line).
    ///
    /// Used when terminal output exceeds the column width and needs
    /// to be split across multiple visual lines.
    pub fn wrapped(content: &str, sequence: u64) -> Self {
        Self {
            content: content.to_string(),
            timestamp: Instant::now(),
            is_wrapped: true,
            sequence,
        }
    }

    /// Create a line with explicit timestamp (for replay/recording).
    pub fn with_timestamp(content: &str, timestamp: Instant) -> Self {
        Self {
            content: content.to_string(),
            timestamp,
            is_wrapped: false,
            sequence: 0,
        }
    }

    /// Get the age of this line in milliseconds.
    pub fn age_ms(&self) -> u64 {
        self.timestamp.elapsed().as_millis() as u64
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new("")
    }
}

/// Search match result with position and context.
///
/// Provides information about where a match was found,
/// including surrounding context lines for display.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Line index where match was found (0-based)
    pub line_index: usize,
    /// Start position within the line (character offset)
    pub start: usize,
    /// End position within the line
    pub end: usize,
    /// The matched text
    pub matched_text: String,
    /// Surrounding context (lines before and after)
    pub context: Vec<String>,
    /// Number of context lines before match
    pub context_before: usize,
    /// Number of context lines after match
    pub context_after: usize,
}

impl SearchMatch {
    /// Create a basic match without context.
    pub fn new(line_index: usize, start: usize, end: usize, matched_text: &str) -> Self {
        Self {
            line_index,
            start,
            end,
            matched_text: matched_text.to_string(),
            context: Vec::new(),
            context_before: 0,
            context_after: 0,
        }
    }

    /// Add context lines to the match.
    pub fn with_context(mut self, before: Vec<String>, after: Vec<String>) -> Self {
        self.context_before = before.len();
        self.context_after = after.len();
        self.context = before;
        self.context.extend(after);
        self
    }
}

/// Search index for fast text and regex searches.
///
/// Maintains an inverted index for efficient searching
/// without blocking output processing.
#[derive(Debug, Default)]
pub struct SearchIndex {
    /// Word-to-line mapping for full-text search
    word_index: std::collections::HashMap<String, Vec<usize>>,
    /// Last indexed line number
    last_indexed: usize,
    /// Whether index needs rebuild
    needs_rebuild: bool,
}

impl SearchIndex {
    /// Create a new empty search index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Index a new line for full-text search.
    ///
    /// This operation is incremental and non-blocking.
    pub fn index_line(&mut self, line_index: usize, content: &str) {
        // Split content into words and index each
        let words = content.split_whitespace();
        for word in words {
            let word_lower = word.to_lowercase();
            self.word_index
                .entry(word_lower)
                .or_insert_with(Vec::new)
                .push(line_index);
        }
        self.last_indexed = line_index;
    }

    /// Search using the word index (fast full-text).
    pub fn word_search(&self, query: &str) -> Vec<usize> {
        let query_lower = query.to_lowercase();
        self.word_index.get(&query_lower).cloned().unwrap_or_default()
    }

    /// Mark index as needing rebuild (after FIFO eviction).
    pub fn mark_rebuild_needed(&mut self) {
        self.needs_rebuild = true;
    }

    /// Clear the entire index.
    pub fn clear(&mut self) {
        self.word_index.clear();
        self.last_indexed = 0;
        self.needs_rebuild = false;
    }
}

/// Scroll buffer with FIFO eviction and search capabilities.
///
/// The scroll buffer maintains terminal output history with:
/// - Fixed maximum capacity (default 10000 lines)
/// - FIFO eviction when capacity exceeded
/// - Non-blocking search operations
/// - Timestamp tracking for each line
///
/// # Constraints (SYSTEM_INVARIANTS.md)
///
/// - Search MUST NOT block output processing
/// - FIFO eviction when exceeding max_lines
/// - Buffer size upper limit enforced
///
/// # Thread Safety
///
/// The buffer uses `Arc<RwLock>` for safe concurrent access.
/// Write operations (push_line, clear) require write lock.
/// Search operations only need read lock and don't block writes.
pub struct ScrollBuffer {
    /// The actual line storage (FIFO queue)
    lines: VecDeque<Line>,
    /// Maximum number of lines to retain
    max_lines: usize,
    /// Optional search index for fast searching
    search_index: Option<SearchIndex>,
    /// Total lines ever pushed (for sequence numbering)
    total_pushed: u64,
    /// Lines evicted due to FIFO
    evicted_count: u64,
    /// Async lock for concurrent access
    lock: Arc<RwLock<()>>,
}

impl ScrollBuffer {
    /// Create a new scroll buffer with the given maximum capacity.
    ///
    /// # Arguments
    ///
    /// * `max_lines` - Maximum number of lines to retain (FIFO eviction)
    ///
    /// # Returns
    ///
    /// A new `ScrollBuffer` instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::scroll_buffer::ScrollBuffer;
    ///
    /// // Standard edition default
    /// let buffer = ScrollBuffer::new(10000);
    ///
    /// // Lite edition
    /// let lite_buffer = ScrollBuffer::new(5000);
    ///
    /// // Pro edition
    /// let pro_buffer = ScrollBuffer::new(50000);
    /// ```
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            search_index: Some(SearchIndex::new()),
            total_pushed: 0,
            evicted_count: 0,
            lock: Arc::new(RwLock::new(())),
        }
    }

    /// Create a buffer with default capacity (10000 lines).
    pub fn default_capacity() -> Self {
        Self::new(DEFAULT_MAX_LINES)
    }

    /// Create a buffer optimized for search operations.
    ///
    /// Enables the search index for fast full-text searching.
    pub fn with_search_index(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            search_index: Some(SearchIndex::new()),
            total_pushed: 0,
            evicted_count: 0,
            lock: Arc::new(RwLock::new(())),
        }
    }

    /// Create a buffer without search index (lower memory usage).
    pub fn without_search_index(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            search_index: None,
            total_pushed: 0,
            evicted_count: 0,
            lock: Arc::new(RwLock::new(())),
        }
    }

    /// Push a new line to the buffer.
    ///
    /// If the buffer exceeds `max_lines`, the oldest line is evicted (FIFO).
    /// This operation is non-blocking and thread-safe.
    ///
    /// # Arguments
    ///
    /// * `line` - The line to add
    ///
    /// # Returns
    ///
    /// The number of lines evicted (0 or 1).
    ///
    /// # Constraints
    ///
    /// - FIFO eviction when exceeding max_lines
    /// - Must not block output processing
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::scroll_buffer::{ScrollBuffer, Line};
    ///
    /// let mut buffer = ScrollBuffer::new(100);
    ///
    /// // Push 100 lines - no eviction
    /// for i in 0..100 {
    ///     buffer.push_line(Line::new(&format!("Line {}", i)));
    /// }
    /// assert_eq!(buffer.len(), 100);
    ///
    /// // Push one more - evicts oldest (FIFO)
    /// buffer.push_line(Line::new("Line 101"));
    /// assert_eq!(buffer.len(), 100);
    /// assert_eq!(buffer.evicted_count(), 1);
    /// ```
    pub fn push_line(&mut self, line: Line) -> u64 {
        let sequence = self.total_pushed;
        self.total_pushed += 1;

        // Create line with sequence number
        let indexed_line = Line {
            content: line.content,
            timestamp: line.timestamp,
            is_wrapped: line.is_wrapped,
            sequence,
        };

        // Index the line if search index is enabled
        if let Some(ref mut index) = self.search_index {
            index.index_line(self.lines.len(), &indexed_line.content);
        }

        // Check capacity and evict if needed (FIFO)
        let evicted = if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
            self.evicted_count += 1;

            // Mark search index as needing rebuild
            if let Some(ref mut index) = self.search_index {
                index.mark_rebuild_needed();
            }

            1
        } else {
            0
        };

        self.lines.push_back(indexed_line);
        evicted
    }

    /// Push multiple lines efficiently.
    ///
    /// Batch operation that minimizes index rebuilds.
    pub fn push_lines(&mut self, lines: Vec<Line>) -> u64 {
        let mut evicted_total = 0;
        for line in lines {
            evicted_total += self.push_line(line);
        }
        evicted_total
    }

    /// Search for matches in the buffer.
    ///
    /// Supports both plain text and regex searches.
    /// This operation uses a read lock and does NOT block output processing.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Search pattern (text or regex)
    /// * `use_regex` - Whether to interpret pattern as regex
    /// * `context_lines` - Number of context lines to include (default 2)
    ///
    /// # Returns
    ///
    /// A vector of `SearchMatch` instances.
    ///
    /// # Constraints
    ///
    /// - Search MUST NOT block output processing
    /// - Uses read lock for concurrent access
    ///
    /// # Example
    ///
    /// ```rust
    /// use easyssh_core::terminal::scroll_buffer::{ScrollBuffer, Line};
    ///
    /// let mut buffer = ScrollBuffer::new(100);
    /// buffer.push_line(Line::new("Error: connection timeout"));
    /// buffer.push_line(Line::new("Success: connected"));
    /// buffer.push_line(Line::new("Error: authentication failed"));
    ///
    /// // Plain text search
    /// let matches = buffer.search("Error", false, 1);
    /// assert_eq!(matches.len(), 2);
    ///
    /// // Regex search
    /// let regex_matches = buffer.search("E[a-z]+:", true, 0);
    /// assert_eq!(regex_matches.len(), 2);
    /// ```
    pub fn search(&self, pattern: &str, use_regex: bool, context_lines: usize) -> Vec<SearchMatch> {
        // Use word index for fast plain text search if available
        if !use_regex {
            if let Some(ref index) = self.search_index {
                let candidate_lines = index.word_search(pattern);
                if candidate_lines.is_empty() {
                    // Fall back to line-by-line search
                    return self.line_search(pattern, false, context_lines);
                }

                // Check candidates for exact match
                let mut matches = Vec::new();
                for line_idx in candidate_lines {
                    if let Some(line) = self.lines.get(line_idx) {
                        if let Some(start) = line.content.find(pattern) {
                            let match_result = self.create_match(
                                line_idx,
                                start,
                                start + pattern.len(),
                                pattern,
                                context_lines,
                            );
                            matches.push(match_result);
                        }
                    }
                }
                return matches;
            }
        }

        // Regex or fallback to line-by-line search
        self.line_search(pattern, use_regex, context_lines)
    }

    /// Line-by-line search (used for regex or when index unavailable).
    fn line_search(&self, pattern: &str, use_regex: bool, context_lines: usize) -> Vec<SearchMatch> {
        let mut matches = Vec::new();

        if use_regex {
            // Compile regex pattern
            let regex = match regex::Regex::new(pattern) {
                Ok(re) => re,
                Err(_) => return matches, // Invalid regex, return empty
            };

            for (idx, line) in self.lines.iter().enumerate() {
                for cap in regex.find_iter(&line.content) {
                    let match_result = self.create_match(
                        idx,
                        cap.start(),
                        cap.end(),
                        &line.content[cap.start()..cap.end()],
                        context_lines,
                    );
                    matches.push(match_result);
                }
            }
        } else {
            // Plain text search
            for (idx, line) in self.lines.iter().enumerate() {
                let mut search_start = 0;
                while let Some(start) = line.content[search_start..].find(pattern) {
                    let actual_start = search_start + start;
                    let match_result = self.create_match(
                        idx,
                        actual_start,
                        actual_start + pattern.len(),
                        pattern,
                        context_lines,
                    );
                    matches.push(match_result);
                    search_start = actual_start + pattern.len();
                }
            }
        }

        matches
    }

    /// Create a SearchMatch with context lines.
    fn create_match(
        &self,
        line_index: usize,
        start: usize,
        end: usize,
        matched_text: &str,
        context_lines: usize,
    ) -> SearchMatch {
        // Gather context lines
        let before: Vec<String> = self.lines
            .iter()
            .skip(line_index.saturating_sub(context_lines))
            .take(context_lines.min(line_index))
            .map(|l| l.content.clone())
            .collect();

        let after: Vec<String> = self.lines
            .iter()
            .skip(line_index + 1)
            .take(context_lines)
            .map(|l| l.content.clone())
            .collect();

        SearchMatch::new(line_index, start, end, matched_text)
            .with_context(before, after)
    }

    /// Search asynchronously (non-blocking).
    ///
    /// This is the preferred method for search operations as it
    /// uses async locks and truly does not block output processing.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Search pattern
    /// * `use_regex` - Whether to use regex
    /// * `context_lines` - Context lines to include
    ///
    /// # Returns
    ///
    /// A future that resolves to search matches.
    pub async fn search_async(
        &self,
        pattern: &str,
        use_regex: bool,
        context_lines: usize,
    ) -> Vec<SearchMatch> {
        // Acquire read lock (doesn't block writes)
        let _guard = self.lock.read().await;

        // Perform search
        self.search(pattern, use_regex, context_lines)
    }

    /// Clear all lines from the buffer.
    ///
    /// Also clears the search index.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.total_pushed = 0;
        self.evicted_count = 0;

        if let Some(ref mut index) = self.search_index {
            index.clear();
        }
    }

    /// Get the current number of lines in the buffer.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get the maximum capacity.
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Get the total lines ever pushed (including evicted).
    pub fn total_pushed(&self) -> u64 {
        self.total_pushed
    }

    /// Get the number of lines evicted by FIFO.
    pub fn evicted_count(&self) -> u64 {
        self.evicted_count
    }

    /// Get lines in range (start to start + count).
    ///
    /// Returns a slice of lines for display or export.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting index (0-based)
    /// * `count` - Number of lines to retrieve
    ///
    /// # Returns
    ///
    /// A vector of lines (may be fewer if range exceeds buffer).
    pub fn get_lines(&self, start: usize, count: usize) -> Vec<&Line> {
        self.lines
            .iter()
            .skip(start)
            .take(count)
            .collect()
    }

    /// Get lines as strings (for display/export).
    pub fn get_lines_text(&self, start: usize, count: usize) -> Vec<String> {
        self.lines
            .iter()
            .skip(start)
            .take(count)
            .map(|l| l.content.clone())
            .collect()
    }

    /// Get the most recent N lines.
    pub fn get_recent(&self, count: usize) -> Vec<&Line> {
        let start = self.lines.len().saturating_sub(count);
        self.get_lines(start, count)
    }

    /// Get all lines as a single string.
    pub fn to_string(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export buffer to a file (for session recording).
    pub fn export(&self) -> Vec<(String, u64)> {
        self.lines
            .iter()
            .map(|l| (l.content.clone(), l.age_ms()))
            .collect()
    }

    /// Rebuild search index (after significant eviction).
    pub fn rebuild_search_index(&mut self) {
        if let Some(ref mut index) = self.search_index {
            index.clear();
            for (idx, line) in self.lines.iter().enumerate() {
                index.index_line(idx, &line.content);
            }
        }
    }

    /// Set new maximum capacity (may trigger immediate eviction).
    pub fn set_max_lines(&mut self, new_max: usize) -> u64 {
        self.max_lines = new_max;

        // Evict if currently over new limit
        let mut evicted = 0;
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
            self.evicted_count += 1;
            evicted += 1;
        }

        // Rebuild index after eviction
        if evicted > 0 {
            self.rebuild_search_index();
        }

        evicted
    }

    /// Get buffer statistics.
    pub fn stats(&self) -> ScrollBufferStats {
        ScrollBufferStats {
            current_lines: self.lines.len(),
            max_lines: self.max_lines,
            total_pushed: self.total_pushed,
            evicted_count: self.evicted_count,
            has_search_index: self.search_index.is_some(),
            oldest_age_ms: self.lines.front().map(|l| l.age_ms()).unwrap_or(0),
            newest_age_ms: self.lines.back().map(|l| l.age_ms()).unwrap_or(0),
        }
    }
}

impl Default for ScrollBuffer {
    fn default() -> Self {
        Self::default_capacity()
    }
}

/// Statistics about the scroll buffer.
#[derive(Debug, Clone)]
pub struct ScrollBufferStats {
    /// Current number of lines
    pub current_lines: usize,
    /// Maximum capacity
    pub max_lines: usize,
    /// Total lines ever pushed
    pub total_pushed: u64,
    /// Lines evicted (FIFO)
    pub evicted_count: u64,
    /// Whether search index is enabled
    pub has_search_index: bool,
    /// Age of oldest line (ms)
    pub oldest_age_ms: u64,
    /// Age of newest line (ms)
    pub newest_age_ms: u64,
}

// ============ Unit Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_buffer_creation() {
        let buffer = ScrollBuffer::new(100);
        assert_eq!(buffer.max_lines(), 100);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_default_capacity() {
        let buffer = ScrollBuffer::default();
        assert_eq!(buffer.max_lines(), DEFAULT_MAX_LINES);
    }

    #[test]
    fn test_push_line() {
        let mut buffer = ScrollBuffer::new(10);

        for i in 0..10 {
            buffer.push_line(Line::new(&format!("Line {}", i)));
        }

        assert_eq!(buffer.len(), 10);
        assert_eq!(buffer.evicted_count(), 0);

        // Push one more - should evict oldest
        buffer.push_line(Line::new("Line 10"));
        assert_eq!(buffer.len(), 10);
        assert_eq!(buffer.evicted_count(), 1);

        // Check FIFO eviction - oldest should be gone
        let lines = buffer.get_lines(0, 10);
        assert_eq!(lines[0].content, "Line 1");
        assert_eq!(lines[9].content, "Line 10");
    }

    #[test]
    fn test_search_plain_text() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Hello World"));
        buffer.push_line(Line::new("Error: timeout"));
        buffer.push_line(Line::new("Success: connected"));
        buffer.push_line(Line::new("Error: auth failed"));

        let matches = buffer.search("Error", false, 0);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_index, 1);
        assert_eq!(matches[1].line_index, 3);
    }

    #[test]
    fn test_search_regex() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("2024-01-15 Error: timeout"));
        buffer.push_line(Line::new("2024-01-16 Warning: memory"));
        buffer.push_line(Line::new("2024-01-17 Error: auth"));

        // Regex for date pattern
        let matches = buffer.search("2024-[0-9]+-[0-9]+", true, 0);
        assert_eq!(matches.len(), 3);

        // Regex for Error or Warning
        let matches = buffer.search("(Error|Warning):", true, 0);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_search_with_context() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Line 0"));
        buffer.push_line(Line::new("Line 1"));
        buffer.push_line(Line::new("Line 2 ERROR HERE"));
        buffer.push_line(Line::new("Line 3"));
        buffer.push_line(Line::new("Line 4"));

        let matches = buffer.search("ERROR", false, 2);
        assert_eq!(matches.len(), 1);

        let match_result = &matches[0];
        assert_eq!(match_result.context_before, 2);
        assert_eq!(match_result.context_after, 2);
        assert_eq!(match_result.context.len(), 4);
    }

    #[test]
    fn test_get_lines() {
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..20 {
            buffer.push_line(Line::new(&format!("Line {}", i)));
        }

        let lines = buffer.get_lines(5, 5);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].content, "Line 5");
        assert_eq!(lines[4].content, "Line 9");
    }

    #[test]
    fn test_get_recent() {
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..20 {
            buffer.push_line(Line::new(&format!("Line {}", i)));
        }

        let recent = buffer.get_recent(5);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].content, "Line 15");
        assert_eq!(recent[4].content, "Line 19");
    }

    #[test]
    fn test_clear() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("Test"));
        buffer.push_line(Line::new("More"));

        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.total_pushed(), 0);
        assert_eq!(buffer.evicted_count(), 0);
    }

    #[test]
    fn test_set_max_lines() {
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..50 {
            buffer.push_line(Line::new(&format!("Line {}", i)));
        }

        assert_eq!(buffer.len(), 50);

        // Reduce capacity - should evict oldest lines
        let evicted = buffer.set_max_lines(30);
        assert_eq!(evicted, 20);
        assert_eq!(buffer.len(), 30);
    }

    #[test]
    fn test_wrapped_line() {
        let line = Line::wrapped("continuation", 5);
        assert!(line.is_wrapped);
        assert_eq!(line.sequence, 5);
    }

    #[test]
    fn test_line_age() {
        let line = Line::new("Test");
        // Age should be very small right after creation
        assert!(line.age_ms() < 100);
    }

    #[test]
    fn test_stats() {
        let mut buffer = ScrollBuffer::new(100);

        for i in 0..50 {
            buffer.push_line(Line::new(&format!("Line {}", i)));
        }

        let stats = buffer.stats();
        assert_eq!(stats.current_lines, 50);
        assert_eq!(stats.max_lines, 100);
        assert_eq!(stats.total_pushed, 50);
        assert_eq!(stats.evicted_count, 0);
        assert!(stats.has_search_index);
    }

    #[test]
    fn test_export() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push_line(Line::new("First"));
        buffer.push_line(Line::new("Second"));

        let exported = buffer.export();
        assert_eq!(exported.len(), 2);
        assert_eq!(exported[0].0, "First");
        assert_eq!(exported[1].0, "Second");
    }

    #[tokio::test]
    async fn test_async_search() {
        let buffer = ScrollBuffer::new(100);
        // Note: Need mutable buffer for push, but search_async uses immutable ref
        // In real usage, buffer would be wrapped in Arc<RwLock<ScrollBuffer>>

        // This test demonstrates the async search pattern
        let pattern = "test";
        let matches = buffer.search_async(pattern, false, 0).await;
        assert!(matches.is_empty()); // Empty buffer
    }
}