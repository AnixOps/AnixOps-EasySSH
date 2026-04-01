#![allow(dead_code)]

//! Code Folding - Stub

/// Code folding
pub struct CodeFolding;

impl CodeFolding {
    pub fn new() -> Self {
        Self
    }

    pub fn fold_range(&mut self, _start_line: usize, _end_line: usize) {
        // Stub
    }

    pub fn unfold_range(&mut self, _start_line: usize) {
        // Stub
    }

    pub fn toggle_fold(&mut self, _line: usize) {
        // Stub
    }
}

/// Fold range
#[derive(Clone, Debug)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
    pub collapsed: bool,
}

/// Fold type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldType {
    Function,
    Class,
    Region,
    Comment,
    CodeBlock,
    Heading,
}
