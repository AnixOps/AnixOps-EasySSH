//! Key Bindings
//!
//! This module defines all keyboard shortcuts and their corresponding actions.
//! Supports both vim-style (hjkl) and arrow key navigation.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

/// Actions that can be triggered by key bindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Navigate up
    NavigateUp,
    /// Navigate down
    NavigateDown,
    /// Navigate left
    NavigateLeft,
    /// Navigate right
    NavigateRight,
    /// Jump to first item
    GoToFirst,
    /// Jump to last item
    GoToLast,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Select/confirm current item
    Select,
    /// Go back/cancel current operation
    Back,
    /// Cancel current operation
    Cancel,
    /// Open search mode
    Search,
    /// Create new server
    NewServer,
    /// Edit selected server
    EditServer,
    /// Delete selected server
    DeleteServer,
    /// Duplicate server
    DuplicateServer,
    /// Create new group
    NewGroup,
    /// Edit selected group
    EditGroup,
    /// Delete selected group
    DeleteGroup,
    /// Connect to selected server
    Connect,
    /// Quick connect (connect without confirmation)
    QuickConnect,
    /// Refresh/reload data
    Refresh,
    /// Toggle theme
    ToggleTheme,
    /// Show help
    Help,
    /// Copy to clipboard
    Copy,
    /// Paste from clipboard
    Paste,
}

impl Action {
    /// Get a description of this action for help display
    pub fn description(&self) -> &'static str {
        match self {
            Action::Quit => "Quit application",
            Action::NavigateUp => "Move up",
            Action::NavigateDown => "Move down",
            Action::NavigateLeft => "Move left",
            Action::NavigateRight => "Move right",
            Action::GoToFirst => "Jump to first",
            Action::GoToLast => "Jump to last",
            Action::PageUp => "Page up",
            Action::PageDown => "Page down",
            Action::Select => "Select/confirm",
            Action::Back => "Go back",
            Action::Cancel => "Cancel",
            Action::Search => "Search/filter",
            Action::NewServer => "New server",
            Action::EditServer => "Edit server",
            Action::DeleteServer => "Delete server",
            Action::DuplicateServer => "Duplicate server",
            Action::NewGroup => "New group",
            Action::EditGroup => "Edit group",
            Action::DeleteGroup => "Delete group",
            Action::Connect => "Connect to server",
            Action::QuickConnect => "Quick connect",
            Action::Refresh => "Refresh data",
            Action::ToggleTheme => "Toggle theme",
            Action::Help => "Show help",
            Action::Copy => "Copy",
            Action::Paste => "Paste",
        }
    }

    /// Get the default key for this action
    pub fn default_key(&self) -> &'static str {
        match self {
            Action::Quit => "q",
            Action::NavigateUp => "↑/k",
            Action::NavigateDown => "↓/j",
            Action::NavigateLeft => "←/h",
            Action::NavigateRight => "→/l",
            Action::GoToFirst => "g/Home",
            Action::GoToLast => "G/End",
            Action::PageUp => "PgUp",
            Action::PageDown => "PgDn",
            Action::Select => "Enter",
            Action::Back => "Esc",
            Action::Cancel => "Esc",
            Action::Search => "/",
            Action::NewServer => "n",
            Action::EditServer => "e",
            Action::DeleteServer => "d",
            Action::DuplicateServer => "y",
            Action::NewGroup => "g",
            Action::EditGroup => "E",
            Action::DeleteGroup => "D",
            Action::Connect => "c/Enter",
            Action::QuickConnect => "Space",
            Action::Refresh => "r/F5",
            Action::ToggleTheme => "t",
            Action::Help => "?/F1",
            Action::Copy => "Ctrl+c",
            Action::Paste => "Ctrl+v",
        }
    }
}

/// Key bindings configuration
pub struct KeyBindings {
    /// Map from key events to actions
    bindings: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation - Arrow keys
        bindings.insert((KeyCode::Up, KeyModifiers::NONE), Action::NavigateUp);
        bindings.insert((KeyCode::Down, KeyModifiers::NONE), Action::NavigateDown);
        bindings.insert((KeyCode::Left, KeyModifiers::NONE), Action::NavigateLeft);
        bindings.insert((KeyCode::Right, KeyModifiers::NONE), Action::NavigateRight);

        // Navigation - Vim style (hjkl)
        bindings.insert(
            (KeyCode::Char('h'), KeyModifiers::NONE),
            Action::NavigateLeft,
        );
        bindings.insert(
            (KeyCode::Char('j'), KeyModifiers::NONE),
            Action::NavigateDown,
        );
        bindings.insert((KeyCode::Char('k'), KeyModifiers::NONE), Action::NavigateUp);
        bindings.insert(
            (KeyCode::Char('l'), KeyModifiers::NONE),
            Action::NavigateRight,
        );

        // Navigation - First/Last (vim style)
        bindings.insert(
            (KeyCode::Char('g'), KeyModifiers::NONE),
            Action::GoToFirst,
        );
        bindings.insert(
            (KeyCode::Char('G'), KeyModifiers::SHIFT),
            Action::GoToLast,
        );

        // Navigation - Home/End/PgUp/PgDn
        bindings.insert((KeyCode::Home, KeyModifiers::NONE), Action::GoToFirst);
        bindings.insert((KeyCode::End, KeyModifiers::NONE), Action::GoToLast);
        bindings.insert((KeyCode::PageUp, KeyModifiers::NONE), Action::PageUp);
        bindings.insert((KeyCode::PageDown, KeyModifiers::NONE), Action::PageDown);

        // Actions
        bindings.insert((KeyCode::Enter, KeyModifiers::NONE), Action::Select);
        bindings.insert((KeyCode::Esc, KeyModifiers::NONE), Action::Cancel);
        bindings.insert((KeyCode::Char('q'), KeyModifiers::NONE), Action::Quit);
        bindings.insert((KeyCode::Char('Q'), KeyModifiers::SHIFT), Action::Quit);

        // Search
        bindings.insert((KeyCode::Char('/'), KeyModifiers::NONE), Action::Search);
        bindings.insert((KeyCode::Char('f'), KeyModifiers::NONE), Action::Search);

        // Server operations
        bindings.insert((KeyCode::Char('n'), KeyModifiers::NONE), Action::NewServer);
        bindings.insert((KeyCode::Char('e'), KeyModifiers::NONE), Action::EditServer);
        bindings.insert(
            (KeyCode::Char('d'), KeyModifiers::NONE),
            Action::DeleteServer,
        );
        bindings.insert(
            (KeyCode::Char('y'), KeyModifiers::NONE),
            Action::DuplicateServer,
        );
        bindings.insert((KeyCode::Char('c'), KeyModifiers::NONE), Action::Connect);
        bindings.insert((KeyCode::Char('C'), KeyModifiers::SHIFT), Action::Connect);
        bindings.insert((KeyCode::Char(' '), KeyModifiers::NONE), Action::QuickConnect);

        // Group operations
        bindings.insert((KeyCode::Char('g'), KeyModifiers::CONTROL), Action::NewGroup);
        bindings.insert((KeyCode::Char('G'), KeyModifiers::SHIFT), Action::EditGroup);
        bindings.insert(
            (KeyCode::Char('D'), KeyModifiers::SHIFT),
            Action::DeleteGroup,
        );

        // Utility
        bindings.insert((KeyCode::Char('r'), KeyModifiers::NONE), Action::Refresh);
        bindings.insert((KeyCode::F(5), KeyModifiers::NONE), Action::Refresh);
        bindings.insert((KeyCode::Char('t'), KeyModifiers::NONE), Action::ToggleTheme);
        bindings.insert((KeyCode::F(1), KeyModifiers::NONE), Action::Help);
        bindings.insert((KeyCode::Char('?'), KeyModifiers::SHIFT), Action::Help);
        bindings.insert((KeyCode::Char('?'), KeyModifiers::NONE), Action::Help);

        // Clipboard
        bindings.insert(
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
            Action::Copy,
        );
        bindings.insert(
            (KeyCode::Char('v'), KeyModifiers::CONTROL),
            Action::Paste,
        );

        Self { bindings }
    }
}

impl KeyBindings {
    /// Get the action for a key event
    pub fn get_action(&self, key: KeyEvent) -> Option<Action> {
        self.bindings.get(&(key.code, key.modifiers)).copied()
    }

    /// Get all bindings for help display
    pub fn get_all_bindings(&self) -> Vec<(Action, String)> {
        let mut result: Vec<(Action, String)> = self
            .bindings
            .iter()
            .map(|((code, mods), action)| {
                let key_str = format_key(*code, *mods);
                (*action, key_str)
            })
            .collect();

        // Deduplicate by action, keeping the shorter key representation
        result.sort_by(|a, b| {
            let action_cmp = a.0.description().cmp(b.0.description());
            if action_cmp != std::cmp::Ordering::Equal {
                action_cmp
            } else {
                a.1.len().cmp(&b.1.len())
            }
        });

        result.dedup_by(|a, b| a.0 == b.0);
        result.sort_by(|a, b| a.0.description().cmp(b.0.description()));

        result
    }

    /// Get all unique actions with their default keys
    pub fn get_help_entries() -> Vec<(Action, &'static str, &'static str)> {
        vec![
            // Navigation
            (Action::NavigateUp, "k/↑", "Move selection up"),
            (Action::NavigateDown, "j/↓", "Move selection down"),
            (Action::NavigateLeft, "h/←", "Move to sidebar"),
            (Action::NavigateRight, "l/→", "Move to details"),
            (Action::GoToFirst, "g/Home", "Jump to first item"),
            (Action::GoToLast, "G/End", "Jump to last item"),
            (Action::PageUp, "PgUp", "Page up"),
            (Action::PageDown, "PgDn", "Page down"),
            (Action::Select, "Enter", "Select/confirm"),
            (Action::Cancel, "Esc", "Cancel/go back"),
            // Server operations
            (Action::NewServer, "n", "Add new server"),
            (Action::EditServer, "e", "Edit server"),
            (Action::DuplicateServer, "y", "Duplicate server"),
            (Action::DeleteServer, "d", "Delete server"),
            (Action::Connect, "c/Enter", "Connect to server"),
            (Action::QuickConnect, "Space", "Quick connect"),
            (Action::Search, "/f", "Search servers"),
            (Action::Refresh, "r/F5", "Refresh data"),
            // Group operations
            (Action::NewGroup, "Ctrl+g", "Add new group"),
            (Action::EditGroup, "G", "Edit group"),
            (Action::DeleteGroup, "D", "Delete group"),
            // Other
            (Action::ToggleTheme, "t", "Toggle theme"),
            (Action::Help, "?/F1", "Show this help"),
            (Action::Quit, "q", "Quit application"),
        ]
    }
}

/// Format a key code and modifiers as a string
fn format_key(code: KeyCode, mods: KeyModifiers) -> String {
    let mut result = String::new();

    if mods.contains(KeyModifiers::CONTROL) {
        result.push_str("Ctrl+");
    }
    if mods.contains(KeyModifiers::ALT) {
        result.push_str("Alt+");
    }
    if mods.contains(KeyModifiers::SHIFT) {
        // For letters, Shift is implicit in uppercase
        match code {
            KeyCode::Char(c) if c.is_ascii_uppercase() => {}
            _ => result.push_str("Shift+"),
        }
    }

    match code {
        KeyCode::Char(c) => result.push(c),
        KeyCode::Up => result.push_str("↑"),
        KeyCode::Down => result.push_str("↓"),
        KeyCode::Left => result.push_str("←"),
        KeyCode::Right => result.push_str("→"),
        KeyCode::Enter => result.push_str("Enter"),
        KeyCode::Esc => result.push_str("Esc"),
        KeyCode::Tab => result.push_str("Tab"),
        KeyCode::Backspace => result.push_str("Backspace"),
        KeyCode::Delete => result.push_str("Del"),
        KeyCode::Home => result.push_str("Home"),
        KeyCode::End => result.push_str("End"),
        KeyCode::PageUp => result.push_str("PgUp"),
        KeyCode::PageDown => result.push_str("PgDn"),
        _ => result.push_str(&format!("{:?}", code)),
    }

    result
}
