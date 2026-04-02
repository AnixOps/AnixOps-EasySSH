#![allow(dead_code)]

//! Multi Cursor - Stub

/// Multi cursor
pub struct MultiCursor;

impl MultiCursor {
    pub fn new() -> Self {
        Self
    }

    pub fn add_cursor(&mut self, _line: usize, _column: usize) {
        // Stub
    }

    pub fn clear(&mut self) {
        // Stub
    }
}

/// Cursor
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub position: (usize, usize),
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
}

impl Cursor {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            position: (line, column),
            selection_start: None,
            selection_end: None,
        }
    }
}
