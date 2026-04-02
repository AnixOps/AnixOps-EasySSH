#![allow(dead_code)]

//! Minimap - Stub

/// Minimap
pub struct Minimap;

impl Minimap {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self) {
        // Stub
    }
}

/// Minimap line
#[derive(Clone, Debug)]
pub struct MinimapLine {
    pub color: [u8; 4],
}
