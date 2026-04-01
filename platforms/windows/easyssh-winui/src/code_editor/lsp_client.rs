#![allow(dead_code)]

//! LSP Client - Stub

/// LSP client
pub struct LspClient;

impl LspClient {
    pub fn new() -> Self {
        Self
    }

    pub fn connect(&mut self, _server_path: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn disconnect(&mut self) {
        // Stub
    }

    pub fn is_connected(&self) -> bool {
        false
    }
}

/// LSP request
#[derive(Clone, Debug)]
pub enum LspRequest {
    Initialize,
    Completion,
    Hover,
    Definition,
    References,
}

/// LSP response
#[derive(Clone, Debug)]
pub enum LspResponse {
    Initialize,
    Completion(Vec<CompletionItem>),
    Hover(String),
    Definition(Location),
    References(Vec<Location>),
}

/// Completion item
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

/// Location
#[derive(Clone, Debug)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Range
#[derive(Clone, Debug)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Position
#[derive(Clone, Debug)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}
