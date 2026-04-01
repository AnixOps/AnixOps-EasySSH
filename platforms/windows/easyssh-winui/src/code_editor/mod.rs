//! Code Editor - Simplified Stub
//!
//! This is a minimal stub implementation for the code editor.

use eframe::egui;

pub mod syntax_highlighter;
pub mod code_folding;
pub mod multi_cursor;
pub mod minimap;
pub mod find_replace;
pub mod diff_view;
pub mod lsp_client;
pub mod editor_theme;
pub mod remote_editor;
pub mod embedded_terminal;

pub use syntax_highlighter::Language;
pub use editor_theme::{EditorTheme, ThemeManager};

/// Code editor
pub struct CodeEditor {
    pub theme_manager: ThemeManager,
    pub show_minimap: bool,
    pub show_terminal: bool,
    pub show_find_replace: bool,
}

impl CodeEditor {
    pub fn new() -> Self {
        Self {
            theme_manager: ThemeManager::new(),
            show_minimap: true,
            show_terminal: false,
            show_find_replace: false,
        }
    }

    pub fn save(&mut self, _path: &str) -> anyhow::Result<()> {
        // Stub
        Ok(())
    }

    pub fn load_content(&mut self, _path: &str, _content: &str) {
        // Stub
    }

    pub fn add_line_to_terminal(&mut self, _line: &str) {
        // Stub
    }

    pub fn render(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.label("Code editor is not available in this build.")
    }

    pub fn open_file(&mut self, _path: &str) {
        // Stub
    }

    pub fn close_file(&mut self, _path: &str) {
        // Stub
    }

    pub fn save_file(&mut self, _path: &str) {
        // Stub
    }

    pub fn set_theme(&mut self, _theme: EditorTheme) {
        // Stub
    }

    pub fn get_theme(&self) -> EditorTheme {
        EditorTheme::dark()
    }

    pub fn undo(&mut self) {
        // Stub
    }

    pub fn redo(&mut self) {
        // Stub
    }

    pub fn cut(&mut self) {
        // Stub
    }

    pub fn copy(&mut self) {
        // Stub
    }

    pub fn paste(&mut self) {
        // Stub
    }

    pub fn select_all(&mut self) {
        // Stub
    }

    pub fn find(&mut self, _query: &str) {
        // Stub
    }

    pub fn replace(&mut self, _query: &str, _replacement: &str) {
        // Stub
    }

    pub fn goto_line(&mut self, _line: usize) {
        // Stub
    }

    pub fn toggle_comment(&mut self) {
        // Stub
    }

    pub fn format_code(&mut self) {
        // Stub
    }
}

/// File info
#[derive(Clone, Default)]
pub struct FileInfo {
    pub path: String,
    pub content: String,
    pub language: Language,
    pub modified: bool,
    pub is_remote: bool,
    pub is_dirty: bool,
    pub line_count: usize,
    pub char_count: usize,
    pub encoding: String,
    pub line_ending: String,
}
