#![allow(dead_code)]

//! Embedded Terminal - Stub

use std::collections::VecDeque;

/// Editor terminal
pub struct EditorTerminal {
    pub lines: VecDeque<TerminalLine>,
    pub max_lines: usize,
}

impl EditorTerminal {
    pub fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines: 1000,
        }
    }

    pub fn write(&mut self, text: &str) {
        self.lines.push_back(TerminalLine {
            text: text.to_string(),
            timestamp: std::time::Instant::now(),
        });

        if self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

/// Terminal line
#[derive(Clone, Debug)]
pub struct TerminalLine {
    pub text: String,
    pub timestamp: std::time::Instant,
}

impl Default for EditorTerminal {
    fn default() -> Self {
        Self::new()
    }
}
