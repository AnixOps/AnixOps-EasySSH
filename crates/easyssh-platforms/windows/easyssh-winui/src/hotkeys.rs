#![allow(dead_code)]
#![allow(dead_code)]

//! Professional Hotkey System for EasySSH Windows
//!
//! Features:
//! - Global hotkeys (window not focused): Ctrl+Alt+T (quick connect), Ctrl+Alt+N (new window)
//! - App hotkeys: Ctrl+K (command palette), Ctrl+Shift+F (search), Ctrl+T (new tab), etc.
//! - Configurable key bindings with conflict detection
//! - VS Code-style Command Palette
//!
//! Uses Windows RegisterHotKey API for global shortcuts

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, WPARAM};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};

/// Unique identifier for a hotkey action
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    // Global hotkeys
    QuickConnectLast,    // Ctrl+Alt+T
    NewConnectionWindow, // Ctrl+Alt+N

    // App hotkeys - Tabs
    NewTab,     // Ctrl+T
    CloseTab,   // Ctrl+W
    NextTab,    // Ctrl+Tab
    PrevTab,    // Ctrl+Shift+Tab
    SwitchTab1, // Ctrl+1
    SwitchTab2, // Ctrl+2
    SwitchTab3, // Ctrl+3
    SwitchTab4, // Ctrl+4
    SwitchTab5, // Ctrl+5
    SwitchTab6, // Ctrl+6
    SwitchTab7, // Ctrl+7
    SwitchTab8, // Ctrl+8
    SwitchTab9, // Ctrl+9

    // App hotkeys - UI
    CommandPalette,   // Ctrl+K
    GlobalSearch,     // Ctrl+Shift+F
    ToggleFullscreen, // F11

    // App hotkeys - Terminal
    TerminalZoomIn,    // Ctrl+Plus
    TerminalZoomOut,   // Ctrl+Minus
    TerminalZoomReset, // Ctrl+0
    TerminalCopy,      // Ctrl+C (when terminal focused)
    TerminalPaste,     // Ctrl+V (when terminal focused)
    TerminalClear,     // Ctrl+L

    // App hotkeys - Navigation
    FocusServers,     // Ctrl+Shift+S
    FocusTerminal,    // Ctrl+Shift+T
    FocusFileBrowser, // Ctrl+Shift+B
    ToggleSidebar,    // Ctrl+B

    // App hotkeys - Snippets
    OpenSnippets,  // Ctrl+Shift+P
    InsertSnippet, // Ctrl+Shift+Space

    // App hotkeys - Split Layout
    SplitHorizontal, // Ctrl+Shift+H - Split current panel horizontally
    SplitVertical,   // Ctrl+Shift+V - Split current panel vertically
    ClosePanel,      // Ctrl+Shift+W - Close current panel
    NextPanel,       // Alt+Right - Switch to next panel
    PrevPanel,       // Alt+Left - Switch to previous panel

    // Custom user-defined
    Custom(String),
}

impl HotkeyAction {
    pub fn display_name(&self) -> String {
        match self {
            Self::QuickConnectLast => "Quick Connect to Last Server".to_string(),
            Self::NewConnectionWindow => "New Connection Window".to_string(),
            Self::NewTab => "New Tab".to_string(),
            Self::CloseTab => "Close Tab".to_string(),
            Self::NextTab => "Next Tab".to_string(),
            Self::PrevTab => "Previous Tab".to_string(),
            Self::SwitchTab1 => "Switch to Tab 1".to_string(),
            Self::SwitchTab2 => "Switch to Tab 2".to_string(),
            Self::SwitchTab3 => "Switch to Tab 3".to_string(),
            Self::SwitchTab4 => "Switch to Tab 4".to_string(),
            Self::SwitchTab5 => "Switch to Tab 5".to_string(),
            Self::SwitchTab6 => "Switch to Tab 6".to_string(),
            Self::SwitchTab7 => "Switch to Tab 7".to_string(),
            Self::SwitchTab8 => "Switch to Tab 8".to_string(),
            Self::SwitchTab9 => "Switch to Tab 9".to_string(),
            Self::CommandPalette => "Command Palette".to_string(),
            Self::GlobalSearch => "Global Search".to_string(),
            Self::ToggleFullscreen => "Toggle Fullscreen".to_string(),
            Self::TerminalZoomIn => "Terminal Zoom In".to_string(),
            Self::TerminalZoomOut => "Terminal Zoom Out".to_string(),
            Self::TerminalZoomReset => "Terminal Zoom Reset".to_string(),
            Self::TerminalCopy => "Copy (Terminal)".to_string(),
            Self::TerminalPaste => "Paste (Terminal)".to_string(),
            Self::TerminalClear => "Clear Terminal".to_string(),
            Self::FocusServers => "Focus Server List".to_string(),
            Self::FocusTerminal => "Focus Terminal".to_string(),
            Self::FocusFileBrowser => "Focus File Browser".to_string(),
            Self::ToggleSidebar => "Toggle Sidebar".to_string(),
            Self::OpenSnippets => "Open Snippets".to_string(),
            Self::InsertSnippet => "Insert Snippet".to_string(),
            Self::SplitHorizontal => "Split Panel Horizontally".to_string(),
            Self::SplitVertical => "Split Panel Vertically".to_string(),
            Self::ClosePanel => "Close Current Panel".to_string(),
            Self::NextPanel => "Next Panel".to_string(),
            Self::PrevPanel => "Previous Panel".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::QuickConnectLast | Self::NewConnectionWindow => "Global",
            Self::NewTab
            | Self::CloseTab
            | Self::NextTab
            | Self::PrevTab
            | Self::SwitchTab1
            | Self::SwitchTab2
            | Self::SwitchTab3
            | Self::SwitchTab4
            | Self::SwitchTab5
            | Self::SwitchTab6
            | Self::SwitchTab7
            | Self::SwitchTab8
            | Self::SwitchTab9 => "Tabs",
            Self::CommandPalette
            | Self::GlobalSearch
            | Self::ToggleFullscreen
            | Self::FocusServers
            | Self::FocusTerminal
            | Self::FocusFileBrowser
            | Self::ToggleSidebar => "Navigation",
            Self::TerminalZoomIn
            | Self::TerminalZoomOut
            | Self::TerminalZoomReset
            | Self::TerminalCopy
            | Self::TerminalPaste
            | Self::TerminalClear => "Terminal",
            Self::OpenSnippets | Self::InsertSnippet => "Snippets",
            Self::SplitHorizontal
            | Self::SplitVertical
            | Self::ClosePanel
            | Self::NextPanel
            | Self::PrevPanel => "Layout",
            Self::Custom(_) => "Custom",
        }
    }

    pub fn default_hotkey(&self) -> Option<KeyBinding> {
        match self {
            // Global hotkeys
            Self::QuickConnectLast => {
                Some(KeyBinding::new(vec![Key::Control, Key::Alt, Key::T]).global(true))
            }
            Self::NewConnectionWindow => {
                Some(KeyBinding::new(vec![Key::Control, Key::Alt, Key::N]).global(true))
            }

            // App hotkeys - Tabs
            Self::NewTab => Some(KeyBinding::new(vec![Key::Control, Key::T])),
            Self::CloseTab => Some(KeyBinding::new(vec![Key::Control, Key::W])),
            Self::NextTab => Some(KeyBinding::new(vec![Key::Control, Key::Tab])),
            Self::PrevTab => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::Tab])),
            Self::SwitchTab1 => Some(KeyBinding::new(vec![Key::Control, Key::Num1])),
            Self::SwitchTab2 => Some(KeyBinding::new(vec![Key::Control, Key::Num2])),
            Self::SwitchTab3 => Some(KeyBinding::new(vec![Key::Control, Key::Num3])),
            Self::SwitchTab4 => Some(KeyBinding::new(vec![Key::Control, Key::Num4])),
            Self::SwitchTab5 => Some(KeyBinding::new(vec![Key::Control, Key::Num5])),
            Self::SwitchTab6 => Some(KeyBinding::new(vec![Key::Control, Key::Num6])),
            Self::SwitchTab7 => Some(KeyBinding::new(vec![Key::Control, Key::Num7])),
            Self::SwitchTab8 => Some(KeyBinding::new(vec![Key::Control, Key::Num8])),
            Self::SwitchTab9 => Some(KeyBinding::new(vec![Key::Control, Key::Num9])),

            // App hotkeys - UI
            Self::CommandPalette => Some(KeyBinding::new(vec![Key::Control, Key::K])),
            Self::GlobalSearch => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::F])),
            Self::ToggleFullscreen => Some(KeyBinding::new(vec![Key::F11])),

            // App hotkeys - Terminal
            Self::TerminalZoomIn => Some(KeyBinding::new(vec![Key::Control, Key::Plus])),
            Self::TerminalZoomOut => Some(KeyBinding::new(vec![Key::Control, Key::Minus])),
            Self::TerminalZoomReset => Some(KeyBinding::new(vec![Key::Control, Key::Num0])),
            Self::TerminalClear => Some(KeyBinding::new(vec![Key::Control, Key::L])),

            // App hotkeys - Navigation
            Self::FocusServers => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::S])),
            Self::FocusTerminal => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::T])),
            Self::FocusFileBrowser => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::B])),
            Self::ToggleSidebar => Some(KeyBinding::new(vec![Key::Control, Key::B])),

            // App hotkeys - Snippets
            Self::OpenSnippets => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::P])),
            Self::InsertSnippet => {
                Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::Space]))
            }

            // App hotkeys - Split Layout
            Self::SplitHorizontal => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::H])),
            Self::SplitVertical => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::V])),
            Self::ClosePanel => Some(KeyBinding::new(vec![Key::Control, Key::Shift, Key::W])),
            Self::NextPanel => Some(KeyBinding::new(vec![Key::Alt, Key::Right])),
            Self::PrevPanel => Some(KeyBinding::new(vec![Key::Alt, Key::Left])),

            // Terminal copy/paste handled specially (context-aware)
            Self::TerminalCopy | Self::TerminalPaste => None,

            Self::Custom(_) => None,
        }
    }
}

/// A single key
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    // Modifiers
    Control,
    Alt,
    Shift,
    Win,
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // Numbers (top row)
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Special keys
    Tab,
    Space,
    Enter,
    Escape,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    // Arrow keys
    Up,
    Down,
    Left,
    Right,
    // Symbols
    Plus,
    Minus,
    Equals,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,
}

impl Key {
    pub fn display_name(&self) -> String {
        match self {
            Self::Control => "Ctrl".to_string(),
            Self::Alt => "Alt".to_string(),
            Self::Shift => "Shift".to_string(),
            Self::Win => "Win".to_string(),
            Self::Num0 => "0".to_string(),
            Self::Num1 => "1".to_string(),
            Self::Num2 => "2".to_string(),
            Self::Num3 => "3".to_string(),
            Self::Num4 => "4".to_string(),
            Self::Num5 => "5".to_string(),
            Self::Num6 => "6".to_string(),
            Self::Num7 => "7".to_string(),
            Self::Num8 => "8".to_string(),
            Self::Num9 => "9".to_string(),
            Self::Numpad0 => "Num 0".to_string(),
            Self::Numpad1 => "Num 1".to_string(),
            Self::Numpad2 => "Num 2".to_string(),
            Self::Numpad3 => "Num 3".to_string(),
            Self::Numpad4 => "Num 4".to_string(),
            Self::Numpad5 => "Num 5".to_string(),
            Self::Numpad6 => "Num 6".to_string(),
            Self::Numpad7 => "Num 7".to_string(),
            Self::Numpad8 => "Num 8".to_string(),
            Self::Numpad9 => "Num 9".to_string(),
            Self::Plus => "+".to_string(),
            Self::Minus => "-".to_string(),
            Self::Equals => "=".to_string(),
            Self::PageUp => "PgUp".to_string(),
            Self::PageDown => "PgDn".to_string(),
            Self::Grave => "`".to_string(),
            _ => format!("{:?}", self),
        }
    }

    /// Get virtual key code for Windows API
    #[cfg(windows)]
    pub fn to_vk_code(&self) -> u32 {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        match self {
            // Letters
            Self::A => VK_A.0 as u32,
            Self::B => VK_B.0 as u32,
            Self::C => VK_C.0 as u32,
            Self::D => VK_D.0 as u32,
            Self::E => VK_E.0 as u32,
            Self::F => VK_F.0 as u32,
            Self::G => VK_G.0 as u32,
            Self::H => VK_H.0 as u32,
            Self::I => VK_I.0 as u32,
            Self::J => VK_J.0 as u32,
            Self::K => VK_K.0 as u32,
            Self::L => VK_L.0 as u32,
            Self::M => VK_M.0 as u32,
            Self::N => VK_N.0 as u32,
            Self::O => VK_O.0 as u32,
            Self::P => VK_P.0 as u32,
            Self::Q => VK_Q.0 as u32,
            Self::R => VK_R.0 as u32,
            Self::S => VK_S.0 as u32,
            Self::T => VK_T.0 as u32,
            Self::U => VK_U.0 as u32,
            Self::V => VK_V.0 as u32,
            Self::W => VK_W.0 as u32,
            Self::X => VK_X.0 as u32,
            Self::Y => VK_Y.0 as u32,
            Self::Z => VK_Z.0 as u32,
            // Numbers
            Self::Num0 => VK_0.0 as u32,
            Self::Num1 => VK_1.0 as u32,
            Self::Num2 => VK_2.0 as u32,
            Self::Num3 => VK_3.0 as u32,
            Self::Num4 => VK_4.0 as u32,
            Self::Num5 => VK_5.0 as u32,
            Self::Num6 => VK_6.0 as u32,
            Self::Num7 => VK_7.0 as u32,
            Self::Num8 => VK_8.0 as u32,
            Self::Num9 => VK_9.0 as u32,
            // Function keys
            Self::F1 => VK_F1.0 as u32,
            Self::F2 => VK_F2.0 as u32,
            Self::F3 => VK_F3.0 as u32,
            Self::F4 => VK_F4.0 as u32,
            Self::F5 => VK_F5.0 as u32,
            Self::F6 => VK_F6.0 as u32,
            Self::F7 => VK_F7.0 as u32,
            Self::F8 => VK_F8.0 as u32,
            Self::F9 => VK_F9.0 as u32,
            Self::F10 => VK_F10.0 as u32,
            Self::F11 => VK_F11.0 as u32,
            Self::F12 => VK_F12.0 as u32,
            // Special keys
            Self::Tab => VK_TAB.0 as u32,
            Self::Space => VK_SPACE.0 as u32,
            Self::Enter => VK_RETURN.0 as u32,
            Self::Escape => VK_ESCAPE.0 as u32,
            Self::Backspace => VK_BACK.0 as u32,
            Self::Delete => VK_DELETE.0 as u32,
            Self::Insert => VK_INSERT.0 as u32,
            Self::Home => VK_HOME.0 as u32,
            Self::End => VK_END.0 as u32,
            Self::PageUp => VK_PRIOR.0 as u32,
            Self::PageDown => VK_NEXT.0 as u32,
            // Arrow keys
            Self::Up => VK_UP.0 as u32,
            Self::Down => VK_DOWN.0 as u32,
            Self::Left => VK_LEFT.0 as u32,
            Self::Right => VK_RIGHT.0 as u32,
            // Symbols
            Self::Plus => VK_OEM_PLUS.0 as u32,
            Self::Minus => VK_OEM_MINUS.0 as u32,
            Self::Equals => VK_OEM_PLUS.0 as u32, // Same as Plus on most keyboards
            Self::Grave => VK_OEM_3.0 as u32,
            _ => 0,
        }
    }
}

/// A complete key binding (combination of keys)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    pub keys: Vec<Key>,
    pub global: bool,
    pub when_focused: bool,
}

impl KeyBinding {
    pub fn new(keys: Vec<Key>) -> Self {
        Self {
            keys,
            global: false,
            when_focused: true,
        }
    }

    pub fn global(mut self, is_global: bool) -> Self {
        self.global = is_global;
        self
    }

    pub fn when_focused(mut self, focused: bool) -> Self {
        self.when_focused = focused;
        self
    }

    /// Check if this is currently pressed in egui input state
    pub fn is_pressed(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            let mut pressed = true;
            for key in &self.keys {
                let is_down = match key {
                    Key::Control => i.modifiers.ctrl,
                    Key::Alt => i.modifiers.alt,
                    Key::Shift => i.modifiers.shift,
                    Key::Win => i.modifiers.command,
                    Key::A => i.key_down(egui::Key::A),
                    Key::B => i.key_down(egui::Key::B),
                    Key::C => i.key_down(egui::Key::C),
                    Key::D => i.key_down(egui::Key::D),
                    Key::E => i.key_down(egui::Key::E),
                    Key::F => i.key_down(egui::Key::F),
                    Key::G => i.key_down(egui::Key::G),
                    Key::H => i.key_down(egui::Key::H),
                    Key::I => i.key_down(egui::Key::I),
                    Key::J => i.key_down(egui::Key::J),
                    Key::K => i.key_down(egui::Key::K),
                    Key::L => i.key_down(egui::Key::L),
                    Key::M => i.key_down(egui::Key::M),
                    Key::N => i.key_down(egui::Key::N),
                    Key::O => i.key_down(egui::Key::O),
                    Key::P => i.key_down(egui::Key::P),
                    Key::Q => i.key_down(egui::Key::Q),
                    Key::R => i.key_down(egui::Key::R),
                    Key::S => i.key_down(egui::Key::S),
                    Key::T => i.key_down(egui::Key::T),
                    Key::U => i.key_down(egui::Key::U),
                    Key::V => i.key_down(egui::Key::V),
                    Key::W => i.key_down(egui::Key::W),
                    Key::X => i.key_down(egui::Key::X),
                    Key::Y => i.key_down(egui::Key::Y),
                    Key::Z => i.key_down(egui::Key::Z),
                    Key::Num0 => i.key_down(egui::Key::Num0),
                    Key::Num1 => i.key_down(egui::Key::Num1),
                    Key::Num2 => i.key_down(egui::Key::Num2),
                    Key::Num3 => i.key_down(egui::Key::Num3),
                    Key::Num4 => i.key_down(egui::Key::Num4),
                    Key::Num5 => i.key_down(egui::Key::Num5),
                    Key::Num6 => i.key_down(egui::Key::Num6),
                    Key::Num7 => i.key_down(egui::Key::Num7),
                    Key::Num8 => i.key_down(egui::Key::Num8),
                    Key::Num9 => i.key_down(egui::Key::Num9),
                    Key::Tab => i.key_down(egui::Key::Tab),
                    Key::Space => i.key_down(egui::Key::Space),
                    Key::Enter => i.key_down(egui::Key::Enter),
                    Key::Escape => i.key_down(egui::Key::Escape),
                    Key::Backspace => i.key_down(egui::Key::Backspace),
                    Key::Delete => i.key_down(egui::Key::Delete),
                    Key::Insert => i.key_down(egui::Key::Insert),
                    Key::Home => i.key_down(egui::Key::Home),
                    Key::End => i.key_down(egui::Key::End),
                    Key::PageUp => i.key_down(egui::Key::PageUp),
                    Key::PageDown => i.key_down(egui::Key::PageDown),
                    Key::Up => i.key_down(egui::Key::ArrowUp),
                    Key::Down => i.key_down(egui::Key::ArrowDown),
                    Key::Left => i.key_down(egui::Key::ArrowLeft),
                    Key::Right => i.key_down(egui::Key::ArrowRight),
                    Key::F11 => i.key_down(egui::Key::F11),
                    Key::Plus => i.key_down(egui::Key::Plus),
                    Key::Minus => i.key_down(egui::Key::Minus),
                    _ => false,
                };
                if !is_down {
                    pressed = false;
                    break;
                }
            }
            pressed
        })
    }

    /// Check if this keybinding was just pressed
    pub fn just_pressed(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            let mut just_pressed = true;
            for key in &self.keys {
                let was_pressed = match key {
                    Key::Control => i.modifiers.ctrl,
                    Key::Alt => i.modifiers.alt,
                    Key::Shift => i.modifiers.shift,
                    Key::Win => i.modifiers.command,
                    Key::A => i.key_pressed(egui::Key::A),
                    Key::B => i.key_pressed(egui::Key::B),
                    Key::C => i.key_pressed(egui::Key::C),
                    Key::D => i.key_pressed(egui::Key::D),
                    Key::E => i.key_pressed(egui::Key::E),
                    Key::F => i.key_pressed(egui::Key::F),
                    Key::G => i.key_pressed(egui::Key::G),
                    Key::H => i.key_pressed(egui::Key::H),
                    Key::I => i.key_pressed(egui::Key::I),
                    Key::J => i.key_pressed(egui::Key::J),
                    Key::K => i.key_pressed(egui::Key::K),
                    Key::L => i.key_pressed(egui::Key::L),
                    Key::M => i.key_pressed(egui::Key::M),
                    Key::N => i.key_pressed(egui::Key::N),
                    Key::O => i.key_pressed(egui::Key::O),
                    Key::P => i.key_pressed(egui::Key::P),
                    Key::Q => i.key_pressed(egui::Key::Q),
                    Key::R => i.key_pressed(egui::Key::R),
                    Key::S => i.key_pressed(egui::Key::S),
                    Key::T => i.key_pressed(egui::Key::T),
                    Key::U => i.key_pressed(egui::Key::U),
                    Key::V => i.key_pressed(egui::Key::V),
                    Key::W => i.key_pressed(egui::Key::W),
                    Key::X => i.key_pressed(egui::Key::X),
                    Key::Y => i.key_pressed(egui::Key::Y),
                    Key::Z => i.key_pressed(egui::Key::Z),
                    Key::Num0 => i.key_pressed(egui::Key::Num0),
                    Key::Num1 => i.key_pressed(egui::Key::Num1),
                    Key::Num2 => i.key_pressed(egui::Key::Num2),
                    Key::Num3 => i.key_pressed(egui::Key::Num3),
                    Key::Num4 => i.key_pressed(egui::Key::Num4),
                    Key::Num5 => i.key_pressed(egui::Key::Num5),
                    Key::Num6 => i.key_pressed(egui::Key::Num6),
                    Key::Num7 => i.key_pressed(egui::Key::Num7),
                    Key::Num8 => i.key_pressed(egui::Key::Num8),
                    Key::Num9 => i.key_pressed(egui::Key::Num9),
                    Key::Tab => i.key_pressed(egui::Key::Tab),
                    Key::Space => i.key_pressed(egui::Key::Space),
                    Key::Enter => i.key_pressed(egui::Key::Enter),
                    Key::Escape => i.key_pressed(egui::Key::Escape),
                    Key::Backspace => i.key_pressed(egui::Key::Backspace),
                    Key::Delete => i.key_pressed(egui::Key::Delete),
                    Key::Insert => i.key_pressed(egui::Key::Insert),
                    Key::Home => i.key_pressed(egui::Key::Home),
                    Key::End => i.key_pressed(egui::Key::End),
                    Key::PageUp => i.key_pressed(egui::Key::PageUp),
                    Key::PageDown => i.key_pressed(egui::Key::PageDown),
                    Key::Up => i.key_pressed(egui::Key::ArrowUp),
                    Key::Down => i.key_pressed(egui::Key::ArrowDown),
                    Key::Left => i.key_pressed(egui::Key::ArrowLeft),
                    Key::Right => i.key_pressed(egui::Key::ArrowRight),
                    Key::F11 => i.key_pressed(egui::Key::F11),
                    Key::Plus => i.key_pressed(egui::Key::Plus),
                    Key::Minus => i.key_pressed(egui::Key::Minus),
                    _ => false,
                };
                if !was_pressed {
                    just_pressed = false;
                    break;
                }
            }
            just_pressed
        })
    }

    /// Convert to display string (e.g., "Ctrl+K")
    pub fn display_string(&self) -> String {
        let parts: Vec<String> = self.keys.iter().map(|k| k.display_name()).collect();
        parts.join("+")
    }

    /// Get Windows modifiers for global hotkey registration
    #[cfg(windows)]
    pub fn to_windows_modifiers(&self) -> HOT_KEY_MODIFIERS {
        let mut modifiers = HOT_KEY_MODIFIERS(0);
        for key in &self.keys {
            match key {
                Key::Control => modifiers.0 |= MOD_CONTROL.0,
                Key::Alt => modifiers.0 |= MOD_ALT.0,
                Key::Shift => modifiers.0 |= MOD_SHIFT.0,
                Key::Win => modifiers.0 |= MOD_WIN.0,
                _ => {}
            }
        }
        modifiers
    }

    /// Get the main key (non-modifier) for global hotkey registration
    #[cfg(windows)]
    pub fn to_windows_key(&self) -> u32 {
        for key in &self.keys {
            match key {
                Key::Control | Key::Alt | Key::Shift | Key::Win => continue,
                _ => return key.to_vk_code(),
            }
        }
        0
    }
}

/// A command that can be executed from the Command Palette
#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub action: HotkeyAction,
    pub label: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub shortcut: Option<KeyBinding>,
    pub execute: Arc<dyn Fn() + Send + Sync>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("id", &self.id)
            .field("action", &self.action)
            .field("label", &self.label)
            .field("description", &self.description)
            .field("category", &self.category)
            .field("icon", &self.icon)
            .field("shortcut", &self.shortcut)
            .field("execute", &"<closure>")
            .finish()
    }
}

/// Hotkey event callback type
pub type HotkeyCallback = Arc<Mutex<dyn FnMut(HotkeyAction) + Send + Sync>>;

/// Main hotkey manager
pub struct HotkeyManager {
    /// Mapping from action to key binding
    bindings: HashMap<HotkeyAction, KeyBinding>,
    /// Reverse mapping for conflict detection
    reverse_bindings: HashMap<KeyBinding, HotkeyAction>,
    /// Global hotkey IDs registered with Windows
    #[cfg(windows)]
    global_hotkeys: HashMap<i32, HotkeyAction>,
    /// Next global hotkey ID
    #[cfg(windows)]
    next_hotkey_id: i32,
    /// Window handle for global hotkey registration
    #[cfg(windows)]
    hwnd: Option<HWND>,
    /// Original WNDPROC for subclassing
    #[cfg(windows)]
    original_wndproc: Option<isize>,
    /// Callback when hotkey is triggered
    callback: Option<HotkeyCallback>,
    /// Registered commands for Command Palette
    commands: Vec<Command>,
    /// Last server for quick connect
    last_server_id: Option<String>,
    /// Terminal font zoom level
    terminal_zoom: f32,
    /// Whether terminal has focus
    terminal_focused: bool,
}

// Safety: HotkeyManager contains raw pointers (HWND, WNDPROC) but they are only
// accessed from the main thread. The Send/Sync impls are safe because we ensure
// thread safety through the Arc<Mutex<>> wrapper in practice.
#[cfg(windows)]
unsafe impl Send for HotkeyManager {}
#[cfg(windows)]
unsafe impl Sync for HotkeyManager {}

impl HotkeyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: HashMap::new(),
            reverse_bindings: HashMap::new(),
            #[cfg(windows)]
            global_hotkeys: HashMap::new(),
            #[cfg(windows)]
            next_hotkey_id: 1000,
            #[cfg(windows)]
            hwnd: None,
            #[cfg(windows)]
            original_wndproc: None,
            callback: None,
            commands: Vec::new(),
            last_server_id: None,
            terminal_zoom: 1.0,
            terminal_focused: false,
        };

        // Load default bindings
        manager.load_default_bindings();

        manager
    }

    /// Load all default key bindings
    fn load_default_bindings(&mut self) {
        let actions = vec![
            HotkeyAction::QuickConnectLast,
            HotkeyAction::NewConnectionWindow,
            HotkeyAction::NewTab,
            HotkeyAction::CloseTab,
            HotkeyAction::NextTab,
            HotkeyAction::PrevTab,
            HotkeyAction::SwitchTab1,
            HotkeyAction::SwitchTab2,
            HotkeyAction::SwitchTab3,
            HotkeyAction::SwitchTab4,
            HotkeyAction::SwitchTab5,
            HotkeyAction::SwitchTab6,
            HotkeyAction::SwitchTab7,
            HotkeyAction::SwitchTab8,
            HotkeyAction::SwitchTab9,
            HotkeyAction::CommandPalette,
            HotkeyAction::GlobalSearch,
            HotkeyAction::ToggleFullscreen,
            HotkeyAction::TerminalZoomIn,
            HotkeyAction::TerminalZoomOut,
            HotkeyAction::TerminalZoomReset,
            HotkeyAction::TerminalClear,
            HotkeyAction::FocusServers,
            HotkeyAction::FocusTerminal,
            HotkeyAction::FocusFileBrowser,
            HotkeyAction::ToggleSidebar,
            HotkeyAction::OpenSnippets,
            HotkeyAction::InsertSnippet,
        ];

        for action in actions {
            if let Some(binding) = action.default_hotkey() {
                let _ = self.register_binding(action, binding);
            }
        }
    }

    /// Register a key binding for an action
    pub fn register_binding(
        &mut self,
        action: HotkeyAction,
        binding: KeyBinding,
    ) -> Result<(), String> {
        // Check for conflicts
        if let Some(existing_action) = self.reverse_bindings.get(&binding) {
            if *existing_action != action {
                return Err(format!(
                    "Conflict: {} is already bound to {}",
                    binding.display_string(),
                    existing_action.display_name()
                ));
            }
        }

        // Remove old binding for this action
        if let Some(old_binding) = self.bindings.remove(&action) {
            self.reverse_bindings.remove(&old_binding);

            // Unregister global hotkey if needed
            #[cfg(windows)]
            if old_binding.global {
                self.unregister_global_hotkey(&action);
            }
        }

        // Register new binding
        self.bindings.insert(action.clone(), binding.clone());
        self.reverse_bindings
            .insert(binding.clone(), action.clone());

        // Register global hotkey if needed
        #[cfg(windows)]
        if binding.global {
            self.register_global_hotkey(&action, &binding)?;
        }

        info!(
            "Registered hotkey: {} -> {}",
            binding.display_string(),
            action.display_name()
        );
        Ok(())
    }

    /// Remove a key binding
    pub fn remove_binding(&mut self, action: &HotkeyAction) {
        if let Some(binding) = self.bindings.remove(action) {
            self.reverse_bindings.remove(&binding);
            #[cfg(windows)]
            if binding.global {
                self.unregister_global_hotkey(action);
            }
        }
    }

    /// Get the binding for an action
    pub fn get_binding(&self, action: &HotkeyAction) -> Option<&KeyBinding> {
        self.bindings.get(action)
    }

    /// Check for conflicts with a proposed binding
    pub fn check_conflict(
        &self,
        binding: &KeyBinding,
        exclude: Option<&HotkeyAction>,
    ) -> Option<HotkeyAction> {
        if let Some(action) = self.reverse_bindings.get(binding) {
            if let Some(exclude) = exclude {
                if action != exclude {
                    return Some(action.clone());
                }
            } else {
                return Some(action.clone());
            }
        }
        None
    }

    /// Set the callback for hotkey events
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut(HotkeyAction) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(Mutex::new(callback)));
    }

    /// Process input and trigger hotkeys
    pub fn process_input(&mut self, ctx: &egui::Context) -> Vec<HotkeyAction> {
        let mut triggered = Vec::new();

        for (action, binding) in &self.bindings {
            if binding.global {
                continue; // Global hotkeys are handled by Windows
            }

            if binding.just_pressed(ctx) {
                triggered.push(action.clone());

                // Execute callback
                if let Some(callback) = &self.callback {
                    if let Ok(mut cb) = callback.lock() {
                        cb(action.clone());
                    }
                }
            }
        }

        triggered
    }

    /// Set window handle for global hotkey registration (Windows)
    #[cfg(windows)]
    pub fn set_window_handle(&mut self, hwnd: HWND) {
        self.hwnd = Some(hwnd);

        // Collect global hotkeys first to avoid borrow issues
        let global_hotkeys: Vec<(HotkeyAction, KeyBinding)> = self
            .bindings
            .iter()
            .filter(|(_, binding)| binding.global)
            .map(|(action, binding)| (action.clone(), binding.clone()))
            .collect();

        // Register all global hotkeys
        for (action, binding) in global_hotkeys {
            if let Err(e) = self.register_global_hotkey(&action, &binding) {
                error!(
                    "Failed to register global hotkey {}: {}",
                    action.display_name(),
                    e
                );
            }
        }
    }

    /// Register a global hotkey with Windows
    #[cfg(windows)]
    fn register_global_hotkey(
        &mut self,
        action: &HotkeyAction,
        binding: &KeyBinding,
    ) -> Result<(), String> {
        let hwnd = self.hwnd.ok_or("No window handle set")?;
        let id = self.next_hotkey_id;
        self.next_hotkey_id += 1;

        let modifiers = binding.to_windows_modifiers();
        let vk = binding.to_windows_key();

        if vk == 0 {
            return Err("No key specified in binding".to_string());
        }

        unsafe {
            match RegisterHotKey(Some(hwnd), id, modifiers, vk) {
                Ok(_) => {
                    self.global_hotkeys.insert(id, action.clone());
                    info!(
                        "Registered global hotkey: {} (id={})",
                        action.display_name(),
                        id
                    );
                    Ok(())
                }
                Err(e) => Err(format!("RegisterHotKey failed: {:?}", e)),
            }
        }
    }

    /// Unregister a global hotkey
    #[cfg(windows)]
    fn unregister_global_hotkey(&mut self, action: &HotkeyAction) {
        let hwnd = match self.hwnd {
            Some(h) => h,
            None => return,
        };

        let ids_to_remove: Vec<i32> = self
            .global_hotkeys
            .iter()
            .filter(|(_, a)| a == &action)
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_remove {
            unsafe {
                let _ = UnregisterHotKey(Some(hwnd), id);
            }
            self.global_hotkeys.remove(&id);
            debug!("Unregistered global hotkey id={}", id);
        }
    }

    /// Handle WM_HOTKEY message from Windows
    #[cfg(windows)]
    pub fn handle_hotkey_message(&mut self, wparam: WPARAM) -> bool {
        let id = wparam.0 as i32;

        if let Some(action) = self.global_hotkeys.get(&id) {
            info!("Global hotkey triggered: {}", action.display_name());

            // Execute callback
            if let Some(callback) = &self.callback {
                if let Ok(mut cb) = callback.lock() {
                    cb(action.clone());
                }
            }
            return true;
        }

        false
    }

    /// Set the last server ID for quick connect
    pub fn set_last_server(&mut self, server_id: String) {
        self.last_server_id = Some(server_id);
    }

    /// Get the last server ID
    pub fn get_last_server(&self) -> Option<&String> {
        self.last_server_id.as_ref()
    }

    /// Get terminal zoom level
    pub fn get_terminal_zoom(&self) -> f32 {
        self.terminal_zoom
    }

    /// Set terminal zoom level
    pub fn set_terminal_zoom(&mut self, zoom: f32) {
        self.terminal_zoom = zoom.clamp(0.5, 3.0);
    }

    /// Adjust terminal zoom
    pub fn adjust_terminal_zoom(&mut self, delta: f32) {
        self.terminal_zoom = (self.terminal_zoom + delta).clamp(0.5, 3.0);
    }

    /// Reset terminal zoom
    pub fn reset_terminal_zoom(&mut self) {
        self.terminal_zoom = 1.0;
    }

    /// Set terminal focus state
    pub fn set_terminal_focused(&mut self, focused: bool) {
        self.terminal_focused = focused;
    }

    /// Check if terminal is focused
    pub fn is_terminal_focused(&self) -> bool {
        self.terminal_focused
    }

    /// Get all bindings for settings UI
    pub fn get_all_bindings(&self) -> &HashMap<HotkeyAction, KeyBinding> {
        &self.bindings
    }

    /// Save configuration to file
    pub fn save_config(&self) -> Result<String, serde_json::Error> {
        let config: HashMap<String, KeyBinding> = self
            .bindings
            .iter()
            .map(|(action, binding)| (format!("{:?}", action), binding.clone()))
            .collect();
        serde_json::to_string_pretty(&config)
    }

    /// Load configuration from JSON string
    pub fn load_config(&mut self, json: &str) -> Result<(), String> {
        let config: HashMap<String, KeyBinding> =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse config: {}", e))?;

        // Clear existing bindings first
        self.bindings.clear();
        self.reverse_bindings.clear();

        for (action_str, binding) in config {
            // Parse action string back to enum
            if let Some(action) = Self::parse_action_from_string(&action_str) {
                // Register the binding without conflict checking (we're restoring)
                self.bindings.insert(action.clone(), binding.clone());
                self.reverse_bindings
                    .insert(binding.clone(), action.clone());

                // Register global hotkey if needed
                #[cfg(windows)]
                if binding.global {
                    if let Some(hwnd) = self.hwnd {
                        let id = self.next_hotkey_id;
                        self.next_hotkey_id += 1;
                        let modifiers = binding.to_windows_modifiers();
                        let vk = binding.to_windows_key();
                        if vk != 0 {
                            unsafe {
                                if RegisterHotKey(Some(hwnd), id, modifiers, vk).is_ok() {
                                    self.global_hotkeys.insert(id, action.clone());
                                    info!(
                                        "Restored global hotkey: {} (id={})",
                                        action.display_name(),
                                        id
                                    );
                                }
                            }
                        }
                    }
                }

                info!("Loaded binding: {} -> {:?}", action.display_name(), binding);
            } else {
                warn!("Unknown action in config: {}", action_str);
            }
        }

        info!("Loaded {} hotkey bindings from config", self.bindings.len());
        Ok(())
    }

    /// Parse action string back to HotkeyAction enum
    fn parse_action_from_string(s: &str) -> Option<HotkeyAction> {
        // Parse the debug format like "QuickConnectLast" or "Custom(\"action_name\")"
        let s = s.trim();

        if s.starts_with("Custom(") {
            // Extract the custom name from Custom("name")
            if let Some(start) = s.find('"') {
                if let Some(end) = s.rfind('"') {
                    if start < end {
                        let name = &s[start + 1..end];
                        return Some(HotkeyAction::Custom(name.to_string()));
                    }
                }
            }
            return None;
        }

        // Match standard actions
        match s {
            "QuickConnectLast" => Some(HotkeyAction::QuickConnectLast),
            "NewConnectionWindow" => Some(HotkeyAction::NewConnectionWindow),
            "NewTab" => Some(HotkeyAction::NewTab),
            "CloseTab" => Some(HotkeyAction::CloseTab),
            "NextTab" => Some(HotkeyAction::NextTab),
            "PrevTab" => Some(HotkeyAction::PrevTab),
            "SwitchTab1" => Some(HotkeyAction::SwitchTab1),
            "SwitchTab2" => Some(HotkeyAction::SwitchTab2),
            "SwitchTab3" => Some(HotkeyAction::SwitchTab3),
            "SwitchTab4" => Some(HotkeyAction::SwitchTab4),
            "SwitchTab5" => Some(HotkeyAction::SwitchTab5),
            "SwitchTab6" => Some(HotkeyAction::SwitchTab6),
            "SwitchTab7" => Some(HotkeyAction::SwitchTab7),
            "SwitchTab8" => Some(HotkeyAction::SwitchTab8),
            "SwitchTab9" => Some(HotkeyAction::SwitchTab9),
            "CommandPalette" => Some(HotkeyAction::CommandPalette),
            "GlobalSearch" => Some(HotkeyAction::GlobalSearch),
            "ToggleFullscreen" => Some(HotkeyAction::ToggleFullscreen),
            "TerminalZoomIn" => Some(HotkeyAction::TerminalZoomIn),
            "TerminalZoomOut" => Some(HotkeyAction::TerminalZoomOut),
            "TerminalZoomReset" => Some(HotkeyAction::TerminalZoomReset),
            "TerminalCopy" => Some(HotkeyAction::TerminalCopy),
            "TerminalPaste" => Some(HotkeyAction::TerminalPaste),
            "TerminalClear" => Some(HotkeyAction::TerminalClear),
            "FocusServers" => Some(HotkeyAction::FocusServers),
            "FocusTerminal" => Some(HotkeyAction::FocusTerminal),
            "FocusFileBrowser" => Some(HotkeyAction::FocusFileBrowser),
            "ToggleSidebar" => Some(HotkeyAction::ToggleSidebar),
            "OpenSnippets" => Some(HotkeyAction::OpenSnippets),
            "InsertSnippet" => Some(HotkeyAction::InsertSnippet),
            _ => {
                warn!("Unknown HotkeyAction string: {}", s);
                None
            }
        }
    }

    /// Get the configuration file path
    pub fn config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("easyssh").join("hotkeys.json"))
    }

    /// Save configuration to file
    pub fn save_to_file(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = self
            .save_config()
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        std::fs::write(&config_path, content)?;
        info!("Hotkey configuration saved to: {:?}", config_path);
        Ok(())
    }

    /// Load configuration from file
    pub fn load_from_file(&mut self) -> anyhow::Result<()> {
        let config_path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            self.load_config(&content)
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
            info!("Hotkey configuration loaded from: {:?}", config_path);
        } else {
            info!(
                "No hotkey config file found at {:?}, using defaults",
                config_path
            );
        }

        Ok(())
    }

    /// Reset all bindings to defaults and save
    pub fn reset_to_defaults_and_save(&mut self) -> anyhow::Result<()> {
        // Clear all existing bindings
        self.bindings.clear();
        self.reverse_bindings.clear();

        // Unregister all global hotkeys
        #[cfg(windows)]
        {
            if let Some(hwnd) = self.hwnd {
                for id in self.global_hotkeys.keys() {
                    unsafe {
                        let _ = UnregisterHotKey(Some(hwnd), *id);
                    }
                }
            }
            self.global_hotkeys.clear();
        }

        // Load default bindings
        self.load_default_bindings();

        // Save to file
        self.save_to_file()?;

        info!("Hotkeys reset to defaults and saved");
        Ok(())
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            // Unregister all global hotkeys
            if let Some(hwnd) = self.hwnd {
                for id in self.global_hotkeys.keys() {
                    unsafe {
                        let _ = UnregisterHotKey(Some(hwnd), *id);
                    }
                }
            }
        }
    }
}

/// VS Code-style Command Palette
pub struct CommandPalette {
    pub visible: bool,
    pub query: String,
    pub selected_index: usize,
    pub commands: Vec<Command>,
    pub filtered_commands: Vec<usize>, // Indices into commands
    pub scroll_offset: f32,
    pub recent_commands: Vec<String>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected_index: 0,
            commands: Vec::new(),
            filtered_commands: Vec::new(),
            scroll_offset: 0.0,
            recent_commands: Vec::new(),
        }
    }

    /// Register a command
    pub fn register_command(&mut self, command: Command) {
        self.commands.push(command);
        self.filter_commands();
    }

    /// Show the palette
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.filter_commands();

        // Sort by recent usage
        self.sort_by_recency();
    }

    /// Hide the palette
    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Update query and filter commands
    pub fn update_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
        self.filter_commands();
    }

    /// Filter commands based on query
    fn filter_commands(&mut self) {
        let query_lower = self.query.to_lowercase();

        self.filtered_commands = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                let label_match = cmd.label.to_lowercase().contains(&query_lower);
                let desc_match = cmd
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                let category_match = cmd.category.to_lowercase().contains(&query_lower);
                label_match || desc_match || category_match
            })
            .map(|(idx, _)| idx)
            .collect();
    }

    /// Sort commands by recency
    fn sort_by_recency(&mut self) {
        // Recent commands first
        self.filtered_commands.sort_by(|a, b| {
            let a_recent = self
                .recent_commands
                .iter()
                .position(|id| id == &self.commands[*a].id);
            let b_recent = self
                .recent_commands
                .iter()
                .position(|id| id == &self.commands[*b].id);

            match (a_recent, b_recent) {
                (Some(a_pos), Some(b_pos)) => a_pos.cmp(&b_pos),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    /// Select next command
    pub fn select_next(&mut self) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
        }
    }

    /// Select previous command
    pub fn select_prev(&mut self) {
        if !self.filtered_commands.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_commands.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Execute selected command
    pub fn execute_selected(&mut self) -> bool {
        if let Some(&cmd_idx) = self.filtered_commands.get(self.selected_index) {
            let command = &self.commands[cmd_idx];

            // Execute
            (command.execute)();

            // Track in recent commands
            self.track_recent(command.id.clone());

            // Hide palette
            self.hide();

            return true;
        }
        false
    }

    /// Track a recently used command
    fn track_recent(&mut self, command_id: String) {
        // Remove if exists
        self.recent_commands.retain(|id| id != &command_id);

        // Add to front
        self.recent_commands.insert(0, command_id);

        // Keep only last 10
        self.recent_commands.truncate(10);
    }

    /// Get selected command
    pub fn get_selected(&self) -> Option<&Command> {
        self.filtered_commands
            .get(self.selected_index)
            .map(|&idx| &self.commands[idx])
    }

    /// Handle keyboard input in palette
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        let mut handled = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowDown) {
                self.select_next();
                handled = true;
            } else if i.key_pressed(egui::Key::ArrowUp) {
                self.select_prev();
                handled = true;
            } else if i.key_pressed(egui::Key::Enter) {
                self.execute_selected();
                handled = true;
            } else if i.key_pressed(egui::Key::Escape) {
                self.hide();
                handled = true;
            }
        });

        handled
    }

    /// Render the command palette UI
    pub fn render(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if !self.visible {
            return;
        }

        // Modal overlay
        let screen_rect = ctx.screen_rect();

        // Darken background
        ui.painter()
            .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(128));

        // Center palette
        let palette_width = 600.0;
        let palette_max_height = 400.0;
        let palette_x = (screen_rect.width() - palette_width) / 2.0;
        let palette_y = 100.0;

        let palette_rect = egui::Rect::from_min_size(
            egui::pos2(palette_x, palette_y),
            egui::vec2(palette_width, palette_max_height),
        );

        // Palette background
        ui.painter()
            .rect_filled(palette_rect, 8.0, ui.visuals().panel_fill);

        // Shadow
        ui.painter().rect_stroke(
            palette_rect,
            8.0,
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color),
        );

        // Input area
        let input_rect = palette_rect.shrink(16.0);
        let mut query_clone = self.query.clone();

        ui.allocate_ui_at_rect(input_rect, |ui| {
            ui.vertical(|ui| {
                // Search icon + input
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut query_clone)
                            .font(egui::TextStyle::Heading)
                            .hint_text("Type a command...")
                            .desired_width(ui.available_width()),
                    );
                });

                // Update query
                if query_clone != self.query {
                    self.update_query(query_clone);
                }

                ui.separator();

                // Results list
                let mut command_to_execute: Option<usize> = None;
                egui::ScrollArea::vertical()
                    .max_height(palette_max_height - 100.0)
                    .show(ui, |ui| {
                        let filtered: Vec<_> = self.filtered_commands.clone();

                        for (display_idx, &cmd_idx) in filtered.iter().enumerate() {
                            let cmd = &self.commands[cmd_idx];
                            let is_selected = display_idx == self.selected_index;

                            let response = ui.selectable_label(
                                is_selected,
                                format!(
                                    "{} {}  -  {}",
                                    cmd.icon.as_deref().unwrap_or("⚡"),
                                    cmd.label,
                                    cmd.shortcut
                                        .as_ref()
                                        .map(|s| s.display_string())
                                        .unwrap_or_default()
                                ),
                            );

                            if response.clicked() {
                                self.selected_index = display_idx;
                                command_to_execute = Some(cmd_idx);
                            }

                            // Description on hover
                            response
                                .on_hover_text(cmd.description.as_deref().unwrap_or(&cmd.category));
                        }

                        if filtered.is_empty() {
                            ui.label("No matching commands");
                        }
                    });

                // Execute command after the loop to avoid borrow issues
                if let Some(cmd_idx) = command_to_execute {
                    // Execute directly without calling execute_selected to avoid double borrow issues
                    let command = &self.commands[cmd_idx];
                    (command.execute)();
                    self.track_recent(command.id.clone());
                    self.hide();
                }

                // Footer with help
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("↑↓ to navigate, Enter to execute, Esc to close");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{} commands", self.filtered_commands.len()));
                    });
                });
            });
        });

        // Handle keyboard input
        self.handle_input(ctx);

        // Close on click outside
        if ui.input(|i| i.pointer.any_click()) {
            let pointer_pos = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                if !palette_rect.contains(pos) {
                    self.hide();
                }
            }
        }
    }
}

/// Settings UI for configuring hotkeys
pub struct HotkeySettingsUI {
    pub visible: bool,
    pub editing_action: Option<HotkeyAction>,
    pub recording_binding: bool,
    pub recorded_keys: Vec<Key>,
    pub search_query: String,
    pub conflict_warning: Option<String>,
    /// Callback to save configuration when bindings change
    on_save: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl HotkeySettingsUI {
    pub fn new() -> Self {
        Self {
            visible: false,
            editing_action: None,
            recording_binding: false,
            recorded_keys: Vec::new(),
            search_query: String::new(),
            conflict_warning: None,
            on_save: None,
        }
    }

    /// Set the save callback that will be called when bindings change
    pub fn set_save_callback<F>(&mut self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_save = Some(Arc::new(callback));
    }

    /// Trigger the save callback if set
    fn trigger_save(&self) {
        if let Some(callback) = &self.on_save {
            callback();
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.editing_action = None;
        self.recording_binding = false;
        self.recorded_keys.clear();
        self.conflict_warning = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.editing_action = None;
        self.recording_binding = false;
    }

    /// Start recording a new key binding
    pub fn start_recording(&mut self, action: HotkeyAction) {
        self.editing_action = Some(action);
        self.recording_binding = true;
        self.recorded_keys.clear();
        self.conflict_warning = None;
    }

    /// Cancel recording
    pub fn cancel_recording(&mut self) {
        self.recording_binding = false;
        self.recorded_keys.clear();
        self.conflict_warning = None;
    }

    /// Render the settings UI
    pub fn render(&mut self, ctx: &egui::Context, manager: &mut HotkeyManager, _ui: &mut egui::Ui) {
        if !self.visible {
            return;
        }

        egui::Window::new("Keyboard Shortcuts")
            .default_size([500.0, 600.0])
            .show(ctx, |ui| {
                // Search bar
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Search shortcuts..."),
                    );
                });

                ui.separator();

                // Recording overlay
                if self.recording_binding {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Press key combination...");
                            ui.label("Modifiers + key (e.g., Ctrl+Shift+K)");
                            ui.label("");

                            // Show currently recorded keys
                            if !self.recorded_keys.is_empty() {
                                let binding = KeyBinding::new(self.recorded_keys.clone());
                                ui.heading(binding.display_string());
                            }

                            ui.label("");
                            ui.horizontal(|ui| {
                                if ui.button("Cancel").clicked() {
                                    self.cancel_recording();
                                }
                                if !self.recorded_keys.is_empty() && ui.button("Clear").clicked() {
                                    self.recorded_keys.clear();
                                }
                            });
                        });
                    });
                }

                // Conflict warning
                if let Some(warning) = &self.conflict_warning {
                    ui.colored_label(egui::Color32::YELLOW, format!("⚠️ {}", warning));
                }

                // Hotkey list grouped by category
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let bindings: Vec<_> = manager
                        .get_all_bindings()
                        .iter()
                        .filter(|(action, _)| {
                            let query = self.search_query.to_lowercase();
                            action.display_name().to_lowercase().contains(&query)
                        })
                        .map(|(a, b)| (a.clone(), b.clone()))
                        .collect();

                    // Group by category
                    let mut last_category = "";
                    for (action, binding) in bindings {
                        let category = action.category();
                        if category != last_category {
                            ui.heading(category);
                            last_category = category;
                        }

                        ui.horizontal(|ui| {
                            // Action name
                            ui.label(action.display_name())
                                .on_hover_text(action.category());

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Edit button
                                    let is_editing = self.editing_action.as_ref() == Some(&action);
                                    let btn_text = if is_editing && self.recording_binding {
                                        "Recording..."
                                    } else {
                                        &binding.display_string()
                                    };

                                    if ui.button(btn_text).clicked() && !self.recording_binding {
                                        self.start_recording(action.clone());
                                    }

                                    // Reset button
                                    if ui
                                        .small_button("↺")
                                        .on_hover_text("Reset to default")
                                        .clicked()
                                    {
                                        if let Some(default) = action.default_hotkey() {
                                            if manager
                                                .register_binding(action.clone(), default)
                                                .is_ok()
                                            {
                                                self.trigger_save();
                                            }
                                        }
                                    }
                                },
                            );
                        });
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Reset All to Defaults").clicked() {
                        // Clear all and reload defaults
                        for action in manager
                            .get_all_bindings()
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                        {
                            manager.remove_binding(&action);
                        }
                        manager.load_default_bindings();
                        self.trigger_save();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.hide();
                        }
                    });
                });
            });

        // Handle key recording
        if self.recording_binding {
            self.handle_recording(ctx, manager);
        }
    }

    /// Handle key recording input
    fn handle_recording(&mut self, ctx: &egui::Context, manager: &mut HotkeyManager) {
        ctx.input(|i| {
            // Check for escape to cancel
            if i.key_pressed(egui::Key::Escape) {
                self.cancel_recording();
                return;
            }

            // Check for enter to confirm
            if i.key_pressed(egui::Key::Enter) && !self.recorded_keys.is_empty() {
                if let Some(action) = &self.editing_action {
                    let binding = KeyBinding::new(self.recorded_keys.clone());

                    // Check for conflicts
                    if let Some(conflict) = manager.check_conflict(&binding, Some(action)) {
                        self.conflict_warning =
                            Some(format!("Conflicts with: {}", conflict.display_name()));
                    } else {
                        // Apply the binding
                        if let Err(e) = manager.register_binding(action.clone(), binding) {
                            self.conflict_warning = Some(e);
                        } else {
                            self.cancel_recording();
                            self.trigger_save();
                        }
                    }
                }
                return;
            }

            // Detect modifier keys
            let mut keys = Vec::new();

            if i.modifiers.ctrl {
                keys.push(Key::Control);
            }
            if i.modifiers.alt {
                keys.push(Key::Alt);
            }
            if i.modifiers.shift {
                keys.push(Key::Shift);
            }
            if i.modifiers.command {
                keys.push(Key::Win);
            }

            // Detect other keys (only when pressed, not held)
            let key_map = [
                (egui::Key::A, Key::A),
                (egui::Key::B, Key::B),
                (egui::Key::C, Key::C),
                (egui::Key::D, Key::D),
                (egui::Key::E, Key::E),
                (egui::Key::F, Key::F),
                (egui::Key::G, Key::G),
                (egui::Key::H, Key::H),
                (egui::Key::I, Key::I),
                (egui::Key::J, Key::J),
                (egui::Key::K, Key::K),
                (egui::Key::L, Key::L),
                (egui::Key::M, Key::M),
                (egui::Key::N, Key::N),
                (egui::Key::O, Key::O),
                (egui::Key::P, Key::P),
                (egui::Key::Q, Key::Q),
                (egui::Key::R, Key::R),
                (egui::Key::S, Key::S),
                (egui::Key::T, Key::T),
                (egui::Key::U, Key::U),
                (egui::Key::V, Key::V),
                (egui::Key::W, Key::W),
                (egui::Key::X, Key::X),
                (egui::Key::Y, Key::Y),
                (egui::Key::Z, Key::Z),
                (egui::Key::Num0, Key::Num0),
                (egui::Key::Num1, Key::Num1),
                (egui::Key::Num2, Key::Num2),
                (egui::Key::Num3, Key::Num3),
                (egui::Key::Num4, Key::Num4),
                (egui::Key::Num5, Key::Num5),
                (egui::Key::Num6, Key::Num6),
                (egui::Key::Num7, Key::Num7),
                (egui::Key::Num8, Key::Num8),
                (egui::Key::Num9, Key::Num9),
                (egui::Key::Tab, Key::Tab),
                (egui::Key::Space, Key::Space),
                (egui::Key::Enter, Key::Enter),
                (egui::Key::Escape, Key::Escape),
                (egui::Key::Backspace, Key::Backspace),
                (egui::Key::Delete, Key::Delete),
                (egui::Key::Insert, Key::Insert),
                (egui::Key::Home, Key::Home),
                (egui::Key::End, Key::End),
                (egui::Key::PageUp, Key::PageUp),
                (egui::Key::PageDown, Key::PageDown),
                (egui::Key::ArrowUp, Key::Up),
                (egui::Key::ArrowDown, Key::Down),
                (egui::Key::ArrowLeft, Key::Left),
                (egui::Key::ArrowRight, Key::Right),
                (egui::Key::F11, Key::F11),
                (egui::Key::Plus, Key::Plus),
                (egui::Key::Minus, Key::Minus),
            ];

            for (egui_key, our_key) in key_map.iter() {
                if i.key_pressed(*egui_key) {
                    keys.push(*our_key);
                    break; // Only capture one main key
                }
            }

            // Update recorded keys if we detected anything meaningful
            if !keys.is_empty() {
                self.recorded_keys = keys;
            }
        });
    }
}

/// Keyboard Shortcut Cheatsheet - Quick reference for all shortcuts
pub struct ShortcutCheatsheet {
    pub visible: bool,
    pub search_query: String,
    pub selected_category: Option<String>,
}

impl ShortcutCheatsheet {
    pub fn new() -> Self {
        Self {
            visible: false,
            search_query: String::new(),
            selected_category: None,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.search_query.clear();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Render the shortcut cheatsheet
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        manager: &HotkeyManager,
        theme: &crate::design::DesignTheme,
    ) {
        if !self.visible {
            return;
        }

        egui::Window::new("⌨ 快捷键速查表")
            .default_size([500.0, 600.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                // Search bar
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("搜索快捷键...")
                            .desired_width(ui.available_width()),
                    );
                });

                ui.separator();

                // Category filters
                ui.horizontal_wrapped(|ui| {
                    let categories = vec!["全部", "全局", "标签页", "导航", "终端", "代码片段"];
                    for category in categories {
                        let is_selected = self
                            .selected_category
                            .as_ref()
                            .map(|c| c == category)
                            .unwrap_or(category == "全部");

                        let button = if is_selected {
                            crate::design::AccessibleButton::new(theme, category)
                                .style(crate::design::AccessibleButtonStyle::Primary)
                                .build()
                        } else {
                            crate::design::AccessibleButton::new(theme, category)
                                .style(crate::design::AccessibleButtonStyle::Ghost)
                                .build()
                        };

                        if ui.add(button).clicked() {
                            if category == "全部" {
                                self.selected_category = None;
                            } else {
                                self.selected_category = Some(category.to_string());
                            }
                        }
                    }
                });

                ui.separator();

                // Shortcuts list
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_shortcuts_list(ui, manager, theme);
                });

                ui.separator();

                // Footer
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("提示：按 Ctrl+Shift+/ 快速打开此面板")
                            .size(12.0)
                            .color(theme.text_tertiary),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("关闭").clicked() {
                            self.hide();
                        }
                    });
                });
            });
    }

    fn render_shortcuts_list(
        &self,
        ui: &mut egui::Ui,
        manager: &HotkeyManager,
        theme: &crate::design::DesignTheme,
    ) {
        let bindings = manager.get_all_bindings();

        // Group by category
        let mut grouped: std::collections::HashMap<&str, Vec<(&HotkeyAction, &KeyBinding)>> =
            std::collections::HashMap::new();

        for (action, binding) in bindings.iter() {
            let category = action.category();

            // Filter by selected category
            if let Some(ref selected) = self.selected_category {
                let category_matches = match (selected.as_str(), category) {
                    ("全局", "Global") => true,
                    ("标签页", "Tabs") => true,
                    ("导航", "Navigation") => true,
                    ("终端", "Terminal") => true,
                    ("代码片段", "Snippets") => true,
                    _ => false,
                };
                if !category_matches {
                    continue;
                }
            }

            // Filter by search
            if !self.search_query.is_empty() {
                let query = self.search_query.to_lowercase();
                let action_name = action.display_name().to_lowercase();
                let shortcut = binding.display_string().to_lowercase();

                if !action_name.contains(&query) && !shortcut.contains(&query) {
                    continue;
                }
            }

            grouped.entry(category).or_default().push((action, binding));
        }

        // Render grouped shortcuts
        let mut categories: Vec<_> = grouped.keys().collect();
        categories.sort();

        for category in categories {
            ui.collapsing(format!("📂 {}", category), |ui| {
                if let Some(shortcuts) = grouped.get(category) {
                    for (action, binding) in shortcuts.iter() {
                        self.render_shortcut_row(
                            ui,
                            theme,
                            &action.display_name(),
                            &binding.display_string(),
                        );
                    }
                }
            });
        }
    }

    fn render_shortcut_row(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::design::DesignTheme,
        action: &str,
        shortcut: &str,
    ) {
        ui.horizontal(|ui| {
            // Shortcut keys display
            egui::Frame::group(ui.style())
                .fill(theme.bg_tertiary)
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.monospace(shortcut);
                });

            ui.add_space(16.0);

            ui.label(
                egui::RichText::new(action)
                    .size(14.0)
                    .color(theme.text_primary),
            );
        });

        ui.add_space(4.0);
    }
}

/// Keyboard shortcut hints that appear in context menus and tooltips
pub struct ShortcutHint {
    pub action: HotkeyAction,
    pub manager: std::sync::Arc<std::sync::Mutex<HotkeyManager>>,
}

impl ShortcutHint {
    pub fn new(
        action: HotkeyAction,
        manager: std::sync::Arc<std::sync::Mutex<HotkeyManager>>,
    ) -> Self {
        Self { action, manager }
    }

    /// Get the shortcut display string for this action
    pub fn display(&self) -> String {
        if let Ok(mgr) = self.manager.lock() {
            if let Some(binding) = mgr.get_binding(&self.action) {
                format!("({})", binding.display_string())
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Get the shortcut for use in a tooltip
    pub fn tooltip(&self) -> String {
        format!("{} {}", self.action.display_name(), self.display())
    }
}

/// Helper functions for common hotkey operations

/// Check if a modifier combination is pressed
pub fn modifiers_pressed(ctx: &egui::Context, ctrl: bool, alt: bool, shift: bool) -> bool {
    ctx.input(|i| i.modifiers.ctrl == ctrl && i.modifiers.alt == alt && i.modifiers.shift == shift)
}

/// Create a global hotkey-enabled window hook (Windows-specific)
#[cfg(windows)]
pub fn setup_global_hotkeys(hwnd: HWND, manager: &Arc<Mutex<HotkeyManager>>) {
    // Set the window handle in the manager
    if let Ok(mut mgr) = manager.lock() {
        mgr.set_window_handle(hwnd);
    }

    info!("Global hotkeys registered for window {:?}", hwnd);
}
