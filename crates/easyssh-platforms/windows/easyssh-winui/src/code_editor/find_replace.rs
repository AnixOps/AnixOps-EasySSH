#![allow(dead_code)]

//! Find Replace - Stub

/// Find replace
pub struct FindReplace;

impl FindReplace {
    pub fn new() -> Self {
        Self
    }

    pub fn find(&mut self, _query: &str) -> Vec<Match> {
        vec![]
    }

    pub fn replace(&mut self, _replacement: &str) {
        // Stub
    }

    pub fn replace_all(&mut self, _query: &str, _replacement: &str) -> usize {
        0
    }
}

/// Search options
#[derive(Clone, Debug, Default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

/// Match
#[derive(Clone, Debug)]
pub struct Match {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}
