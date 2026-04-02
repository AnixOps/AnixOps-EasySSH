#![allow(dead_code)]

//! Editor Theme - Stub

/// Editor theme
#[derive(Clone, Debug)]
pub struct EditorTheme {
    pub name: String,
}

impl EditorTheme {
    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
        }
    }

    pub fn from_vscode_theme(_path: &str) -> anyhow::Result<Self> {
        Ok(Self::dark())
    }
}

/// Theme manager
pub struct ThemeManager {
    current: EditorTheme,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current: EditorTheme::dark(),
        }
    }

    pub fn current_theme(&self) -> &EditorTheme {
        &self.current
    }

    pub fn load_theme(&mut self, _name: &str) -> Option<EditorTheme> {
        Some(EditorTheme::dark())
    }

    pub fn list_themes(&self) -> Vec<String> {
        vec!["Dark".to_string(), "Light".to_string()]
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
