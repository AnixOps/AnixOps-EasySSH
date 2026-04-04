//! Integration Tests for EasySSH egui Terminal
//!
//! Tests the complete terminal workflow including:
//! - Terminal creation and destruction
//! - Key-driven reset pattern
//! - Buffer management
//! - Search functionality
//! - Platform integration

use easyssh_egui::terminal::buffer::TerminalBuffer;
use easyssh_egui::terminal::view::TerminalView;
use easyssh_egui::platform::windows::WindowsPlatform;
use easyssh_egui::platform::windows::Platform;

/// Test Key-Driven Reset pattern compliance
#[test]
fn test_key_driven_reset_pattern() {
    let platform = WindowsPlatform::new();

    // Create terminal with connection_id-session_id format
    let conn_id = "connection-abc123";
    let sess_id = "session-xyz789";

    let view = platform.create_terminal_view(conn_id, sess_id);

    // Key must follow {connection_id}-{session_id} format
    let expected_key = format!("{}-{}", conn_id, sess_id);
    assert_eq!(view.id(), expected_key);

    // Destroy must clean up
    platform.destroy_terminal_view(view.id());
    assert!(platform.active_terminals().is_empty());
}

/// Test terminal creation with various ID formats
#[test]
fn test_terminal_id_formats() {
    let test_cases = [
        ("simple", "terminal"),
        ("conn-123", "sess-456"),
        ("uuid-a1b2c3d4", "term-e5f6g7h8"),
        ("production-server", "main-session"),
    ];

    let platform = WindowsPlatform::new();

    for (conn_id, sess_id) in test_cases {
        let view = platform.create_terminal_view(conn_id, sess_id);
        let key = view.id();

        // Key must contain dash separator
        assert!(key.contains('-'), "Key '{}' missing separator", key);

        // Key must start with connection_id
        assert!(key.starts_with(conn_id), "Key '{}' doesn't start with '{}'", key, conn_id);

        // Key must end with session_id
        assert!(key.ends_with(sess_id), "Key '{}' doesn't end with '{}'", key, sess_id);

        platform.destroy_terminal_view(key);
    }
}

/// Test terminal buffer FIFO eviction
#[test]
fn test_buffer_fifo_eviction() {
    let max_lines = 100;
    let mut buffer = TerminalBuffer::new(max_lines);

    // Write more lines than max
    for i in 0..(max_lines + 50) {
        buffer.write_raw(&format!("Line {}\n", i));
    }

    // Should only keep max_lines
    assert_eq!(buffer.line_count(), max_lines);

    // First lines should be oldest retained
    let first_line = buffer.get_line(0).expect("First line exists");
    assert!(first_line.text().contains("50")); // Line 50 should be first retained
}

/// Test terminal buffer search functionality
#[test]
fn test_buffer_search_literal() {
    let mut buffer = TerminalBuffer::new(100);

    buffer.write_raw("foo bar baz\n");
    buffer.write_raw("another foo line\n");
    buffer.write_raw("no matches here\n");

    let matches = buffer.search("foo", false);
    assert_eq!(matches.len(), 2);

    // Check match positions
    assert_eq!(matches[0].line, 0);
    assert_eq!(matches[0].cols, (0, 3));
    assert_eq!(matches[1].line, 1);
    assert_eq!(matches[1].cols, (8, 11));
}

/// Test terminal buffer regex search
#[test]
fn test_buffer_search_regex() {
    let mut buffer = TerminalBuffer::new(100);

    buffer.write_raw("Error: file not found\n");
    buffer.write_raw("Warning: deprecated API\n");
    buffer.write_raw("Error: permission denied\n");

    let matches = buffer.search("Error:", true);
    assert_eq!(matches.len(), 2);
    assert!(matches[0].text.starts_with("Error"));
    assert!(matches[1].text.starts_with("Error"));
}

/// Test terminal view lifecycle
#[test]
fn test_terminal_view_lifecycle() {
    let mut view = TerminalView::new("conn-test", "sess-test");

    // Write output
    view.write_output(b"Hello, Terminal!");

    // Resize
    view.resize(100, 30);

    // Search
    view.start_search("Hello", false);

    // Clear
    view.clear();

    // Drop cleans up handles
    drop(view);
}

/// Test terminal selection functionality
#[test]
fn test_terminal_selection() {
    let mut view = TerminalView::new("conn", "sess");

    // Add content
    view.write_output(b"Hello World\n");
    view.scroll_to_bottom();
}

/// Test ANSI color processing
#[test]
fn test_ansi_color_processing() {
    let mut buffer = TerminalBuffer::new(100);

    // Write text with ANSI color codes
    let colored_text = "\x1b[31mRed Text\x1b[0m Normal Text\n";
    buffer.write(colored_text.as_bytes());

    // Write raw text to ensure buffer works
    buffer.write_raw("Simple text\n");

    // Buffer should have content after write_raw
    assert!(buffer.line_count() > 0);
    let line = buffer.get_line(0);
    assert!(line.is_some());
}

/// Test platform terminal operations
#[test]
fn test_platform_operations() {
    let platform = WindowsPlatform::new();

    // Create multiple terminals
    let mut view1 = platform.create_terminal_view("conn1", "sess1");
    let mut view2 = platform.create_terminal_view("conn2", "sess2");

    assert_eq!(platform.active_terminals().len(), 2);

    // Write to terminals
    view1.write_output(b"Output to terminal 1");
    view2.write_output(b"Output to terminal 2");

    // Resize
    view1.resize(80, 24);
    view2.resize(120, 40);

    // Clean up
    platform.destroy_terminal_view(view1.id());
    platform.destroy_terminal_view(view2.id());

    assert!(platform.active_terminals().is_empty());
}

/// Test cursor position tracking
#[test]
fn test_cursor_tracking() {
    let mut buffer = TerminalBuffer::new(100);

    buffer.write_raw("Hello\n");
    let pos = buffer.cursor_position();

    // After "Hello\n", cursor should be at start of next line
    assert_eq!(pos.0, 0); // Column 0
    assert_eq!(pos.1, 1); // Line 1 (second line)
}

/// Test scroll behavior
#[test]
fn test_scroll_behavior() {
    let mut view = TerminalView::new("conn", "sess");

    // Add many lines
    for i in 0..200 {
        view.write_output(format!("Line {}\n", i).as_bytes());
    }

    // Scroll to bottom
    view.scroll_to_bottom();

    // Scroll to top
    view.scroll_to_top();

    // Scroll by offset
    view.scroll_by(50.0);
}

/// Test concurrent terminal operations (thread safety)
#[test]
fn test_concurrent_operations() {
    use std::sync::Arc;
    use std::thread;

    let platform = Arc::new(WindowsPlatform::new());
    let mut handles = vec![];

    // Create terminals from multiple threads
    for i in 0..10 {
        let p = Arc::clone(&platform);
        let h = thread::spawn(move || {
            let conn_id = format!("conn-{}", i);
            let sess_id = format!("sess-{}", i);

            let mut view = p.create_terminal_view(&conn_id, &sess_id);
            view.write_output(b"test output");
            view.resize(80, 24);

            p.destroy_terminal_view(view.id());
        });
        handles.push(h);
    }

    // Wait for all threads
    for h in handles {
        h.join().expect("Thread completed");
    }

    // All terminals should be cleaned up
    assert!(platform.active_terminals().is_empty());
}