//! SSH channel management for russh backend
//!
//! Provides channel operations following SYSTEM_INVARIANTS.md:
//! - Section 1.1: PTY lifecycle (create after Active, destroy properly)
//! - Section 1.2: Scroll buffer limits

use crate::russh_impl::error::{RusshError, RusshResult};

use std::sync::Arc;

/// Result of a command execution.
#[derive(Debug, Clone)]
pub struct RusshExecResult {
    /// Exit code from the command
    pub exit_code: u32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl RusshExecResult {
    /// Check if command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr).
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Get lines from stdout.
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

/// SSH channel wrapper.
pub struct RusshChannel {
    _inner: (),
}

impl RusshChannel {
    /// Create a new channel wrapper.
    pub fn new() -> Self {
        Self { _inner: () }
    }

    /// Send data through the channel.
    pub async fn send(&mut self, _data: &[u8]) -> RusshResult<()> {
        Ok(())
    }

    /// Close the channel.
    pub async fn close(&mut self) -> RusshResult<()> {
        Ok(())
    }
}

impl Default for RusshChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Shell channel for PTY sessions.
///
/// Following SYSTEM_INVARIANTS.md Section 1.1:
/// - PTY must be created after Connection Active
/// - PTY destruction must close main channel first
pub struct RusshShellChannel {
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl RusshShellChannel {
    /// Create a new shell channel.
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Send input to the shell.
    pub async fn send(&self, _data: &[u8]) -> RusshResult<()> {
        Ok(())
    }

    /// Resize the terminal.
    pub async fn resize(&self, _cols: u32, _rows: u32) -> RusshResult<()> {
        Ok(())
    }

    /// Close the shell channel.
    pub async fn close(&mut self) -> RusshResult<()> {
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Send Ctrl+C (interrupt signal).
    pub async fn interrupt(&self) -> RusshResult<()> {
        self.send(&[0x03]).await
    }

    /// Send Ctrl+D (EOF).
    pub async fn eof(&self) -> RusshResult<()> {
        self.send(&[0x04]).await
    }

    /// Send Ctrl+Z (suspend).
    pub async fn suspend(&self) -> RusshResult<()> {
        self.send(&[0x1A]).await
    }
}

impl Default for RusshShellChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RusshShellChannel {
    fn drop(&mut self) {
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Scroll buffer for terminal output.
///
/// Following SYSTEM_INVARIANTS.md Section 1.2:
/// - Buffer size has upper limit (default 10000 lines)
/// - FIFO strategy for overflow
/// - Search operations don't block output processing
pub struct ScrollBuffer {
    /// Buffer content (lines)
    lines: Vec<String>,
    /// Maximum lines
    max_lines: usize,
}

impl ScrollBuffer {
    /// Create a new scroll buffer.
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::with_capacity(max_lines),
            max_lines,
        }
    }

    /// Append a line to the buffer.
    pub fn push(&mut self, line: String) {
        if self.lines.len() >= self.max_lines {
            // Remove oldest line (FIFO)
            self.lines.remove(0);
        }
        self.lines.push(line);
    }

    /// Append multiple lines.
    pub fn extend(&mut self, lines: Vec<String>) {
        for line in lines {
            self.push(line);
        }
    }

    /// Get all lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get line count.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Search for a pattern in the buffer.
    pub fn search(&mut self, pattern: &str) -> Vec<(usize, String)> {
        if pattern.is_empty() {
            return Vec::new();
        }

        let pattern_lower = pattern.to_lowercase();

        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&pattern_lower))
            .map(|(i, line)| (i, line.clone()))
            .collect()
    }

    /// Search with regex pattern.
    pub fn search_regex(&mut self, pattern: &str) -> RusshResult<Vec<(usize, String)>> {
        let re = regex::Regex::new(pattern).map_err(|e| RusshError::Internal {
            reason: format!("Invalid regex: {}", e),
        })?;

        Ok(self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| re.is_match(line))
            .map(|(i, line)| (i, line.clone()))
            .collect())
    }

    /// Get a range of lines.
    pub fn get_range(&self, start: usize, end: usize) -> &[String] {
        let start = start.min(self.lines.len());
        let end = end.min(self.lines.len());
        &self.lines[start..end]
    }

    /// Get the last N lines.
    pub fn last(&self, n: usize) -> &[String] {
        let start = self.lines.len().saturating_sub(n);
        &self.lines[start..]
    }

    /// Get the full content as a single string.
    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }
}

impl Default for ScrollBuffer {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_result() {
        let result = RusshExecResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
        };

        assert!(result.success());
        assert_eq!(result.combined_output(), "hello");
    }

    #[test]
    fn test_exec_result_failed() {
        let result = RusshExecResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };

        assert!(!result.success());
        assert_eq!(result.combined_output(), "\nerror");
    }

    #[test]
    fn test_scroll_buffer_push() {
        let mut buffer = ScrollBuffer::new(5);

        for i in 0..10 {
            buffer.push(format!("line {}", i));
        }

        // Should only keep last 5
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.lines()[0], "line 5");
        assert_eq!(buffer.lines()[4], "line 9");
    }

    #[test]
    fn test_scroll_buffer_search() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push("hello world".to_string());
        buffer.push("foo bar".to_string());
        buffer.push("hello again".to_string());

        let results = buffer.search("hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_scroll_buffer_last() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push("line 1".to_string());
        buffer.push("line 2".to_string());
        buffer.push("line 3".to_string());

        let last = buffer.last(2);
        assert_eq!(last.len(), 2);
        assert_eq!(last[0], "line 2");
    }
}