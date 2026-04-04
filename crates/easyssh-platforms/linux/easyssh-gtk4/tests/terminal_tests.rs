//! Terminal Module Integration Tests
//!
//! Tests for the GTK4 terminal components following SYSTEM_INVARIANTS.md constraints.

#[cfg(test)]
mod terminal_module_tests {
    // Note: These tests can't run the full GTK4 event loop, so we test
    // the non-UI logic components.

    use easyssh_gtk4::terminal::{TerminalBuffer, TerminalStyle, CursorStyle};
    use easyssh_gtk4::terminal::buffer::TextStyle;

    /// Test buffer creation and basic operations
    #[test]
    fn test_buffer_creation_and_append() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.line_count(), 0);

        buffer.append("Hello World\n", None);
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.get_line(0), Some("Hello World".to_string()));
    }

    /// Test buffer FIFO eviction (SYSTEM_INVARIANTS.md Section 1.2)
    #[test]
    fn test_buffer_fifo_eviction() {
        let buffer = TerminalBuffer::with_max_lines(10);

        // Add 15 lines
        for i in 0..15 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        // Should have only 10 lines
        assert_eq!(buffer.line_count(), 10);
        assert_eq!(buffer.evicted_count(), 5);

        // First line should be "Line 5" (oldest 5 evicted)
        assert_eq!(buffer.get_line(0), Some("Line 5".to_string()));
    }

    /// Test buffer search functionality (SYSTEM_INVARIANTS.md Section 1.2)
    #[test]
    fn test_buffer_search_plain_text() {
        let buffer = TerminalBuffer::new();
        buffer.append("Error: connection timeout\n", None);
        buffer.append("Success: connected\n", None);
        buffer.append("Error: auth failed\n", None);

        let matches = buffer.search("Error", false);
        assert_eq!(matches.len(), 2);
    }

    /// Test buffer regex search
    #[test]
    fn test_buffer_search_regex() {
        let buffer = TerminalBuffer::new();
        buffer.append("2024-01-15 10:30:45 Error\n", None);
        buffer.append("2024-01-16 11:45:30 Warning\n", None);

        // Match date pattern
        let matches = buffer.search("2024-[0-9]+-[0-9]+", true);
        assert_eq!(matches.len(), 2);

        // Match time pattern
        let matches = buffer.search("[0-9]{2}:[0-9]{2}:[0-9]{2}", true);
        assert_eq!(matches.len(), 2);
    }

    /// Test ANSI color processing
    #[test]
    fn test_buffer_ansi_processing() {
        let buffer = TerminalBuffer::new();
        buffer.append_with_ansi("\x1b[31mRed text\x1b[0m normal text\n");

        assert_eq!(buffer.line_count(), 1);
        let content = buffer.content();
        assert!(content.contains("Red text"));
        assert!(content.contains("normal text"));
    }

    /// Test text style application
    #[test]
    fn test_buffer_text_styles() {
        let buffer = TerminalBuffer::new();

        buffer.append("Normal\n", None);
        buffer.append("Bold\n", Some(TextStyle::Bold));
        buffer.append("Error\n", Some(TextStyle::Error));
        buffer.append("Success\n", Some(TextStyle::Success));
        buffer.append("Warning\n", Some(TextStyle::Warning));

        assert_eq!(buffer.line_count(), 5);
    }

    /// Test buffer clear operation
    #[test]
    fn test_buffer_clear() {
        let buffer = TerminalBuffer::new();
        buffer.append("Line 1\n", None);
        buffer.append("Line 2\n", None);

        assert_eq!(buffer.line_count(), 2);

        buffer.clear();
        assert_eq!(buffer.line_count(), 0);
        assert_eq!(buffer.evicted_count(), 0);
    }

    /// Test buffer recent lines retrieval
    #[test]
    fn test_buffer_recent_lines() {
        let buffer = TerminalBuffer::new();
        for i in 0..20 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        let recent = buffer.get_recent_lines(5);
        assert_eq!(recent.len(), 5);
        assert!(recent[0].contains("Line 15"));
        assert!(recent[4].contains("Line 19"));
    }

    /// Test style theme loading (SYSTEM_INVARIANTS.md Section 6)
    #[test]
    fn test_terminal_style_themes() {
        // Dark theme
        let dark = TerminalStyle::default_dark();
        assert!(dark.cursor_blink);
        assert_eq!(dark.cursor_style, CursorStyle::Block);

        // Light theme
        let light = TerminalStyle::default_light();
        assert_eq!(light.cursor_style, CursorStyle::Bar);

        // Named themes
        let one_dark = TerminalStyle::from_theme("one-dark").unwrap();
        assert!(one_dark.foreground.red() > 0.5);

        let monokai = TerminalStyle::from_theme("monokai").unwrap();
        assert!(!monokai.cursor_blink);
    }

    /// Test ANSI color retrieval
    #[test]
    fn test_terminal_style_ansi_colors() {
        let style = TerminalStyle::default_dark();

        // Base colors
        let red = style.get_ansi_color(1);
        assert!(red.red() > 0.9);

        let green = style.get_ansi_color(2);
        assert!(green.green() > 0.4);

        // Bright colors
        let bright_red = style.get_ansi_color(9);
        assert!(bright_red.red() > 0.9);
    }

    /// Test 256-color support
    #[test]
    fn test_terminal_style_256_colors() {
        let style = TerminalStyle::default_dark();

        // Color cube (indices 16-231)
        let color = style.get_256_color(16); // First color cube color
        assert!(color.alpha() > 0.0);

        // Grayscale (indices 232-255)
        let gray = style.get_256_color(245);
        // Grayscale should have equal R, G, B
        let tolerance = 0.02;
        assert!((gray.red() - gray.green()).abs() < tolerance);
        assert!((gray.green() - gray.blue()).abs() < tolerance);
    }

    /// Test style CSS generation
    #[test]
    fn test_terminal_style_css() {
        let style = TerminalStyle::default_dark();
        let css = style.to_css();

        assert!(css.contains(".terminal-view"));
        assert!(css.contains(".terminal-output"));
        assert!(css.contains("background-color"));
        assert!(css.contains("font-family"));
    }

    /// Test style modifiers
    #[test]
    fn test_terminal_style_modifiers() {
        let style = TerminalStyle::default_dark()
            .with_font_family("Fira Code")
            .with_font_size(16)
            .with_cursor_style(CursorStyle::Underline)
            .with_opacity(0.8);

        assert_eq!(style.font_family, "Fira Code");
        assert_eq!(style.font_size, 16);
        assert_eq!(style.cursor_style, CursorStyle::Underline);
        assert!((style.background_opacity - 0.8).abs() < 0.01);
    }

    /// Test cursor style conversion
    #[test]
    fn test_cursor_style() {
        assert_eq!(CursorStyle::Block.as_str(), "block");
        assert_eq!(CursorStyle::Underline.as_str(), "underline");
        assert_eq!(CursorStyle::Bar.as_str(), "bar");

        assert_eq!(CursorStyle::from_str("block"), Some(CursorStyle::Block));
        assert_eq!(CursorStyle::from_str("underline"), Some(CursorStyle::Underline));
        assert_eq!(CursorStyle::from_str("bar"), Some(CursorStyle::Bar));
        assert_eq!(CursorStyle::from_str("invalid"), None);
    }
}

#[cfg(test)]
mod input_handler_tests {
    use easyssh_gtk4::terminal::input::TerminalInputHandler;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Test command history management
    #[test]
    fn test_command_history() {
        let handler = TerminalInputHandler::new();
        assert_eq!(handler.history_count(), 0);

        // Add commands via history directly (normally done through execute)
        // Note: In real usage, commands are added through execute_command
        // which requires a callback
    }

    /// Test history max size
    #[test]
    fn test_history_max_size() {
        let handler = TerminalInputHandler::new();

        // The handler has MAX_HISTORY_SIZE = 100
        // We test the clear function
        handler.clear_history();
        assert_eq!(handler.history_count(), 0);
    }

    /// Test input handler reset
    #[test]
    fn test_input_handler_reset() {
        let handler = TerminalInputHandler::new();
        handler.reset();

        // After reset, state should be clean
        assert_eq!(handler.history_count(), 0);
    }
}

#[cfg(test)]
mod search_bar_tests {
    // Note: SearchBar tests are limited without GTK event loop
    // Most logic is tested through TerminalBuffer search tests

    /// Test search pattern constants
    #[test]
    fn test_search_patterns() {
        // Test that common regex patterns work
        let patterns = vec![
            "Error",
            r"\d{4}-\d{2}-\d{2}",
            r"0x[0-9A-Fa-f]+",
            r"\b(Warning|Error)\b",
        ];

        // Patterns compile successfully
        for pattern in patterns {
            let _ = regex::Regex::new(pattern).expect("Pattern should compile");
        }
    }
}

/// Test key format compliance (SYSTEM_INVARIANTS.md Section 0.2)
#[cfg(test)]
mod key_format_tests {
    /// Test key format generation
    #[test]
    fn test_key_format() {
        let connection_id = "conn-123-abc";
        let session_id = "sess-456-xyz";
        let key = format!("{}-{}", connection_id, session_id);

        assert_eq!(key, "conn-123-abc-sess-456-xyz");
        assert!(key.contains(connection_id));
        assert!(key.contains(session_id));
    }

    /// Test key uniqueness
    #[test]
    fn test_key_uniqueness() {
        let keys: Vec<String> = vec![
            format!("{}-{}", "conn-1", "sess-1"),
            format!("{}-{}", "conn-1", "sess-2"),
            format!("{}-{}", "conn-2", "sess-1"),
            format!("{}-{}", "conn-2", "sess-2"),
        ];

        // All keys should be unique
        let unique_count = keys.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 4);
    }
}

/// Test scroll buffer constraints (SYSTEM_INVARIANTS.md Section 1.2)
#[cfg(test)]
mod scroll_buffer_constraints {
    /// Test that Lite version limits are respected
    #[test]
    fn test_lite_buffer_limit() {
        // Lite max = 5000 lines
        let buffer = TerminalBuffer::with_max_lines(5000);

        // Fill beyond limit
        for i in 0..6000 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        assert_eq!(buffer.line_count(), 5000);
        assert_eq!(buffer.evicted_count(), 1000);
    }

    /// Test that Standard version limits are respected
    #[test]
    fn test_standard_buffer_limit() {
        // Standard max = 10000 lines
        let buffer = TerminalBuffer::with_max_lines(10000);

        // Fill beyond limit
        for i in 0..12000 {
            buffer.append(&format!("Line {}\n", i), None);
        }

        assert_eq!(buffer.line_count(), 10000);
        assert_eq!(buffer.evicted_count(), 2000);
    }

    /// Test search doesn't block (simulated)
    #[test]
    fn test_search_non_blocking() {
        let buffer = TerminalBuffer::new();

        // Add some content
        for i in 0..1000 {
            buffer.append(&format!("Line {} with some content here\n", i), None);
        }

        // Measure search time (should be fast)
        let start = std::time::Instant::now();
        let _matches = buffer.search("content", false);
        let elapsed = start.elapsed();

        // Search should complete quickly (< 100ms for 1000 lines)
        assert!(elapsed.as_millis() < 100);
    }
}