//! Enhanced Native Terminal Launcher
//!
//! Provides cross-platform native terminal launching for EasySSH Lite.
//! Features:
//! - Auto-detection of 20+ terminal emulators
//! - User preference persistence
//! - Custom launch parameters per terminal
//! - Smart error handling with suggestions
//! - Platform-optimized launching

use crate::error::LiteError;
use crate::models::server::{AuthMethod, Server};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Terminal preference settings with persistence support
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TerminalPreference {
    /// Auto-detect the best available terminal
    #[default]
    Auto,
    /// Specific terminal by type
    Specific(TerminalType),
    /// Custom terminal command path
    Custom(String),
}

/// Supported terminal emulators across all platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalType {
    // Windows - Modern
    WindowsTerminal,
    WindowsTerminalPreview,
    FluentTerminal,
    // Windows - PowerShell variants
    PowerShell,
    PowerShellCore, // pwsh (PowerShell 7+)
    // Windows - Legacy
    Cmd,
    // Windows - Unix-like
    GitBash,
    Cygwin,
    Msys2,
    // Windows - Modern alternatives
    Hyper,
    Tabby,
    AlacrittyWindows,
    WezTermWindows,
    // Linux - GNOME/GTK
    GnomeTerminal,
    Tilix,
    Terminator,
    // Linux - KDE
    Konsole,
    Yakuake,
    // Linux - XFCE
    Xfce4Terminal,
    // Linux - Lightweight
    Xterm,
    RxvtUnicode,
    St,
    // Linux - Modern GPU
    Alacritty,
    Kitty,
    WezTerm,
    Foot,
    // Linux - Other
    Termite,
    LilyTerm,
    Sakura,
    // macOS - Built-in
    TerminalApp,
    // macOS - Popular alternatives
    ITerm2,
    Warp,
    WezTermMac,
    AlacrittyMac,
    KittyMac,
    Ghostty,
    // macOS - Cross-platform
    HyperMac,
    TabbyMac,
}

impl TerminalType {
    /// Get the executable name for this terminal
    pub fn executable(&self) -> &'static str {
        match self {
            // Windows
            TerminalType::WindowsTerminal => "wt",
            TerminalType::WindowsTerminalPreview => "wt",
            TerminalType::FluentTerminal => "FluentTerminal",
            TerminalType::PowerShell => "powershell",
            TerminalType::PowerShellCore => "pwsh",
            TerminalType::Cmd => "cmd",
            TerminalType::GitBash => "git-bash",
            TerminalType::Cygwin => "mintty",
            TerminalType::Msys2 => "msys2",
            TerminalType::Hyper => "Hyper",
            TerminalType::Tabby => "Tabby",
            TerminalType::AlacrittyWindows => "alacritty",
            TerminalType::WezTermWindows => "wezterm",
            // Linux
            TerminalType::GnomeTerminal => "gnome-terminal",
            TerminalType::Tilix => "tilix",
            TerminalType::Terminator => "terminator",
            TerminalType::Konsole => "konsole",
            TerminalType::Yakuake => "yakuake",
            TerminalType::Xfce4Terminal => "xfce4-terminal",
            TerminalType::Xterm => "xterm",
            TerminalType::RxvtUnicode => "urxvt",
            TerminalType::St => "st",
            TerminalType::Alacritty => "alacritty",
            TerminalType::Kitty => "kitty",
            TerminalType::WezTerm => "wezterm",
            TerminalType::Foot => "foot",
            TerminalType::Termite => "termite",
            TerminalType::LilyTerm => "lilyterm",
            TerminalType::Sakura => "sakura",
            // macOS
            TerminalType::TerminalApp => "Terminal",
            TerminalType::ITerm2 => "iTerm",
            TerminalType::Warp => "Warp",
            TerminalType::WezTermMac => "wezterm",
            TerminalType::AlacrittyMac => "alacritty",
            TerminalType::KittyMac => "kitty",
            TerminalType::Ghostty => "ghostty",
            TerminalType::HyperMac => "Hyper",
            TerminalType::TabbyMac => "Tabby",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TerminalType::WindowsTerminal => "Windows Terminal",
            TerminalType::WindowsTerminalPreview => "Windows Terminal (Preview)",
            TerminalType::FluentTerminal => "Fluent Terminal",
            TerminalType::PowerShell => "Windows PowerShell",
            TerminalType::PowerShellCore => "PowerShell 7+",
            TerminalType::Cmd => "Command Prompt",
            TerminalType::GitBash => "Git Bash",
            TerminalType::Cygwin => "Cygwin Mintty",
            TerminalType::Msys2 => "MSYS2",
            TerminalType::Hyper => "Hyper",
            TerminalType::Tabby => "Tabby",
            TerminalType::AlacrittyWindows => "Alacritty (Windows)",
            TerminalType::WezTermWindows => "WezTerm (Windows)",
            TerminalType::GnomeTerminal => "GNOME Terminal",
            TerminalType::Tilix => "Tilix",
            TerminalType::Terminator => "Terminator",
            TerminalType::Konsole => "Konsole",
            TerminalType::Yakuake => "Yakuake",
            TerminalType::Xfce4Terminal => "XFCE4 Terminal",
            TerminalType::Xterm => "XTerm",
            TerminalType::RxvtUnicode => "RXVT-Unicode",
            TerminalType::St => "Simple Terminal (st)",
            TerminalType::Alacritty => "Alacritty",
            TerminalType::Kitty => "Kitty",
            TerminalType::WezTerm => "WezTerm",
            TerminalType::Foot => "Foot",
            TerminalType::Termite => "Termite",
            TerminalType::LilyTerm => "LilyTerm",
            TerminalType::Sakura => "Sakura",
            TerminalType::TerminalApp => "Terminal.app",
            TerminalType::ITerm2 => "iTerm2",
            TerminalType::Warp => "Warp",
            TerminalType::WezTermMac => "WezTerm",
            TerminalType::AlacrittyMac => "Alacritty",
            TerminalType::KittyMac => "Kitty",
            TerminalType::Ghostty => "Ghostty",
            TerminalType::HyperMac => "Hyper",
            TerminalType::TabbyMac => "Tabby",
        }
    }

    /// Get the platform this terminal belongs to
    pub fn platform(&self) -> Platform {
        match self {
            TerminalType::WindowsTerminal
            | TerminalType::WindowsTerminalPreview
            | TerminalType::FluentTerminal
            | TerminalType::PowerShell
            | TerminalType::PowerShellCore
            | TerminalType::Cmd
            | TerminalType::GitBash
            | TerminalType::Cygwin
            | TerminalType::Msys2
            | TerminalType::Hyper
            | TerminalType::Tabby
            | TerminalType::AlacrittyWindows
            | TerminalType::WezTermWindows => Platform::Windows,
            TerminalType::GnomeTerminal
            | TerminalType::Tilix
            | TerminalType::Terminator
            | TerminalType::Konsole
            | TerminalType::Yakuake
            | TerminalType::Xfce4Terminal
            | TerminalType::Xterm
            | TerminalType::RxvtUnicode
            | TerminalType::St
            | TerminalType::Alacritty
            | TerminalType::Kitty
            | TerminalType::WezTerm
            | TerminalType::Foot
            | TerminalType::Termite
            | TerminalType::LilyTerm
            | TerminalType::Sakura => Platform::Linux,
            TerminalType::TerminalApp
            | TerminalType::ITerm2
            | TerminalType::Warp
            | TerminalType::WezTermMac
            | TerminalType::AlacrittyMac
            | TerminalType::KittyMac
            | TerminalType::Ghostty
            | TerminalType::HyperMac
            | TerminalType::TabbyMac => Platform::MacOS,
        }
    }

    /// Get default priority (lower = higher priority, auto-detect order)
    pub fn default_priority(&self) -> u8 {
        match self {
            // Modern terminals first
            TerminalType::WindowsTerminal => 0,
            TerminalType::Warp => 0,
            TerminalType::ITerm2 => 1,
            TerminalType::WezTerm | TerminalType::WezTermMac | TerminalType::WezTermWindows => 1,
            TerminalType::Kitty | TerminalType::KittyMac => 2,
            TerminalType::Alacritty
            | TerminalType::AlacrittyMac
            | TerminalType::AlacrittyWindows => 3,
            TerminalType::Ghostty => 3,
            // Platform-specific modern
            TerminalType::WindowsTerminalPreview => 4,
            TerminalType::FluentTerminal => 5,
            TerminalType::Hyper | TerminalType::HyperMac => 6,
            TerminalType::Tabby | TerminalType::TabbyMac => 7,
            // Desktop environment defaults
            TerminalType::GnomeTerminal => 8,
            TerminalType::Konsole => 8,
            TerminalType::TerminalApp => 8,
            TerminalType::Tilix => 9,
            TerminalType::Yakuake => 9,
            TerminalType::Terminator => 10,
            TerminalType::Xfce4Terminal => 10,
            // PowerShell variants
            TerminalType::PowerShellCore => 11,
            TerminalType::PowerShell => 12,
            // Cross-platform Unix-like
            TerminalType::GitBash => 13,
            TerminalType::Cygwin => 14,
            TerminalType::Msys2 => 15,
            // Minimal/lightweight
            TerminalType::Foot => 16,
            TerminalType::St => 17,
            TerminalType::Termite => 18,
            TerminalType::Sakura => 19,
            TerminalType::LilyTerm => 20,
            TerminalType::RxvtUnicode => 21,
            TerminalType::Xterm => 22,
            // Legacy
            TerminalType::Cmd => 99,
        }
    }

    /// Get all terminals for current platform
    pub fn platform_terminals() -> Vec<TerminalType> {
        let all = Self::all_terminals();
        all.into_iter()
            .filter(|t| t.platform() == Platform::current())
            .collect()
    }

    /// Get all supported terminals
    pub fn all_terminals() -> Vec<TerminalType> {
        vec![
            // Windows
            TerminalType::WindowsTerminal,
            TerminalType::WindowsTerminalPreview,
            TerminalType::FluentTerminal,
            TerminalType::PowerShell,
            TerminalType::PowerShellCore,
            TerminalType::Cmd,
            TerminalType::GitBash,
            TerminalType::Cygwin,
            TerminalType::Msys2,
            TerminalType::Hyper,
            TerminalType::Tabby,
            TerminalType::AlacrittyWindows,
            TerminalType::WezTermWindows,
            // Linux
            TerminalType::GnomeTerminal,
            TerminalType::Tilix,
            TerminalType::Terminator,
            TerminalType::Konsole,
            TerminalType::Yakuake,
            TerminalType::Xfce4Terminal,
            TerminalType::Xterm,
            TerminalType::RxvtUnicode,
            TerminalType::St,
            TerminalType::Alacritty,
            TerminalType::Kitty,
            TerminalType::WezTerm,
            TerminalType::Foot,
            TerminalType::Termite,
            TerminalType::LilyTerm,
            TerminalType::Sakura,
            // macOS
            TerminalType::TerminalApp,
            TerminalType::ITerm2,
            TerminalType::Warp,
            TerminalType::WezTermMac,
            TerminalType::AlacrittyMac,
            TerminalType::KittyMac,
            TerminalType::Ghostty,
            TerminalType::HyperMac,
            TerminalType::TabbyMac,
        ]
    }

    /// Get installation help for this terminal
    pub fn install_help(&self) -> Option<&'static str> {
        match self {
            TerminalType::WindowsTerminal => {
                Some("Install from Microsoft Store or GitHub releases")
            }
            TerminalType::WindowsTerminalPreview => {
                Some("Install from Microsoft Store (Preview channel)")
            }
            TerminalType::PowerShellCore => {
                Some("Install from GitHub or Microsoft Store: https://aka.ms/powershell")
            }
            TerminalType::FluentTerminal => Some("Install from Microsoft Store"),
            TerminalType::GitBash => {
                Some("Install Git for Windows: https://git-scm.com/download/win")
            }
            TerminalType::Hyper => Some("Download from https://hyper.is"),
            TerminalType::Tabby => Some("Download from https://tabby.sh"),
            TerminalType::Warp => Some("Download from https://warp.dev"),
            TerminalType::ITerm2 => Some("Download from https://iterm2.com"),
            TerminalType::Alacritty
            | TerminalType::AlacrittyMac
            | TerminalType::AlacrittyWindows => Some("Download from https://alacritty.org"),
            TerminalType::Kitty | TerminalType::KittyMac => Some("https://sw.kovidgoyal.net/kitty"),
            TerminalType::WezTerm | TerminalType::WezTermMac | TerminalType::WezTermWindows => {
                Some("https://wezfurlong.org/wezterm")
            }
            TerminalType::Ghostty => Some("https://mitchellh.com/ghostty"),
            TerminalType::Tilix => Some("sudo apt install tilix (Debian/Ubuntu)"),
            TerminalType::Terminator => Some("sudo apt install terminator"),
            TerminalType::Yakuake => Some("sudo apt install yakuake"),
            TerminalType::Foot => Some("sudo apt install foot"),
            _ => None,
        }
    }

    /// Get custom arguments template for this terminal
    pub fn custom_args_template(&self) -> &'static str {
        match self {
            TerminalType::WindowsTerminal => {
                "--profile \"ProfileName\" --startingDirectory \"C:\\\\\""
            }
            TerminalType::PowerShell | TerminalType::PowerShellCore => "-WindowStyle Maximized",
            TerminalType::Cmd => "/T:0A (color scheme)",
            TerminalType::GnomeTerminal => "--theme-variant dark --zoom 1.2",
            TerminalType::Konsole => "--profile \"Default\"",
            TerminalType::Kitty | TerminalType::KittyMac => "--config ~/.config/kitty/ssh.conf",
            TerminalType::Alacritty
            | TerminalType::AlacrittyMac
            | TerminalType::AlacrittyWindows => "--option font.size=14",
            TerminalType::WezTerm | TerminalType::WezTermMac | TerminalType::WezTermWindows => {
                "--config-file ~/.config/wezterm/ssh.lua"
            }
            TerminalType::ITerm2 => "Use AppleScript for advanced options",
            TerminalType::Warp => "Limited CLI support",
            _ => "",
        }
    }
}

/// Operating platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

impl Platform {
    /// Get current platform
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
    }
}

/// Detected terminal information
#[derive(Debug, Clone)]
pub struct DetectedTerminal {
    pub terminal_type: TerminalType,
    pub executable_path: PathBuf,
    pub priority: u8,
    pub version: Option<String>,
}

/// Terminal launch configuration
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Custom arguments for specific terminal types
    pub custom_args: HashMap<TerminalType, Vec<String>>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Keep terminal open after command completes
    pub keep_open: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            custom_args: HashMap::new(),
            env_vars: HashMap::new(),
            working_dir: None,
            keep_open: true,
        }
    }
}

/// Enhanced terminal launcher with preference persistence
pub struct TerminalLauncher {
    preference: TerminalPreference,
    config: TerminalConfig,
    last_detected: Option<Vec<DetectedTerminal>>,
}

impl Default for TerminalLauncher {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalLauncher {
    /// Create a new terminal launcher with default settings
    pub fn new() -> Self {
        Self {
            preference: TerminalPreference::default(),
            config: TerminalConfig::default(),
            last_detected: None,
        }
    }

    /// Create with a specific preference
    pub fn with_preference(preference: TerminalPreference) -> Self {
        Self {
            preference,
            config: TerminalConfig::default(),
            last_detected: None,
        }
    }

    /// Set terminal preference
    pub fn set_preference(&mut self, preference: TerminalPreference) {
        self.preference = preference;
    }

    /// Get current preference
    pub fn preference(&self) -> &TerminalPreference {
        &self.preference
    }

    /// Set configuration
    pub fn with_config(mut self, config: TerminalConfig) -> Self {
        self.config = config;
        self
    }

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut TerminalConfig {
        &mut self.config
    }

    /// Set custom arguments for a specific terminal type
    pub fn set_terminal_args(&mut self, terminal_type: TerminalType, args: Vec<String>) {
        self.config.custom_args.insert(terminal_type, args);
    }

    /// Detect all available terminals on the system with comprehensive search
    pub fn detect_available_terminals(&mut self) -> Vec<DetectedTerminal> {
        let mut detected = Vec::new();

        for terminal_type in TerminalType::platform_terminals() {
            if let Some(info) = self.detect_terminal(terminal_type) {
                detected.push(info);
            }
        }

        // Sort by priority
        detected.sort_by_key(|t| t.priority);

        // Cache results
        self.last_detected = Some(detected.clone());

        detected
    }

    /// Detect a specific terminal with version info
    fn detect_terminal(&self, terminal_type: TerminalType) -> Option<DetectedTerminal> {
        let executable = terminal_type.executable();

        // Try to find executable
        let path = find_executable_advanced(executable, terminal_type)?;

        // Try to get version
        let version = try_get_version(&path, terminal_type);

        Some(DetectedTerminal {
            terminal_type,
            executable_path: path,
            priority: terminal_type.default_priority(),
            version,
        })
    }

    /// Get cached detection results
    pub fn last_detected(&self) -> Option<&Vec<DetectedTerminal>> {
        self.last_detected.as_ref()
    }

    /// Get the best available terminal with detailed error
    pub fn get_best_terminal(&mut self) -> Result<TerminalType, TerminalError> {
        match &self.preference {
            TerminalPreference::Auto => {
                let available = self.detect_available_terminals();
                if available.is_empty() {
                    return Err(TerminalError::NoTerminalFound {
                        platform: Platform::current(),
                        suggestions: self.get_install_suggestions(),
                    });
                }
                Ok(available[0].terminal_type)
            }
            TerminalPreference::Specific(terminal_type) => {
                if self.detect_terminal(*terminal_type).is_some() {
                    Ok(*terminal_type)
                } else {
                    Err(TerminalError::PreferredNotFound {
                        terminal: *terminal_type,
                        install_help: terminal_type.install_help(),
                    })
                }
            }
            TerminalPreference::Custom(_cmd) => {
                // For custom commands, we can't validate easily
                // Just return a placeholder that will be handled specially
                Ok(TerminalType::WindowsTerminal) // Placeholder - custom handling needed
            }
        }
    }

    /// Get installation suggestions for the current platform
    fn get_install_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        match Platform::current() {
            Platform::Windows => vec![
                (
                    "Windows Terminal",
                    "Microsoft Store or https://github.com/microsoft/terminal",
                ),
                ("PowerShell 7", "https://aka.ms/powershell"),
                ("Git Bash", "https://git-scm.com/download/win"),
            ],
            Platform::Linux => vec![
                ("GNOME Terminal", "Usually pre-installed on GNOME"),
                ("Konsole", "Usually pre-installed on KDE"),
                ("Alacritty", "https://alacritty.org"),
                ("Kitty", "https://sw.kovidgoyal.net/kitty"),
            ],
            Platform::MacOS => vec![
                ("Terminal.app", "Built-in, in /Applications/Utilities"),
                ("iTerm2", "https://iterm2.com"),
                ("Warp", "https://warp.dev"),
            ],
        }
    }

    /// Launch terminal and connect to server via SSH
    pub fn launch(&mut self, server: &Server) -> Result<(), TerminalError> {
        let terminal = self.get_best_terminal()?;
        let ssh_cmd = generate_ssh_command(server);

        let title = format!("SSH: {}", server.name);

        self.launch_with_command_ex(&ssh_cmd, Some(&title), terminal, &server.name)
    }

    /// Launch with extended options
    fn launch_with_command_ex(
        &self,
        command: &str,
        title: Option<&str>,
        terminal: TerminalType,
        context: &str,
    ) -> Result<(), TerminalError> {
        let result = match terminal {
            // Windows
            TerminalType::WindowsTerminal | TerminalType::WindowsTerminalPreview => {
                launch_windows_terminal(command, title, &self.config)
            }
            TerminalType::PowerShell => launch_powershell(command, title, &self.config),
            TerminalType::PowerShellCore => launch_pwsh(command, title, &self.config),
            TerminalType::Cmd => launch_cmd(command, title, &self.config),
            TerminalType::GitBash => launch_gitbash(command, title, &self.config),
            TerminalType::FluentTerminal => launch_fluent_terminal(command, title, &self.config),
            TerminalType::Hyper => launch_hyper(command, title, &self.config),
            TerminalType::Tabby => launch_tabby(command, title, &self.config),
            TerminalType::AlacrittyWindows => {
                launch_alacritty_windows(command, title, &self.config)
            }
            TerminalType::WezTermWindows => launch_wezterm_windows(command, title, &self.config),
            TerminalType::Cygwin => launch_cygwin(command, title, &self.config),
            TerminalType::Msys2 => launch_msys2(command, title, &self.config),
            // Linux
            TerminalType::GnomeTerminal => launch_gnome_terminal(command, title, &self.config),
            TerminalType::Tilix => launch_tilix(command, title, &self.config),
            TerminalType::Terminator => launch_terminator(command, title, &self.config),
            TerminalType::Konsole => launch_konsole(command, title, &self.config),
            TerminalType::Yakuake => launch_yakuake(command, title, &self.config),
            TerminalType::Xfce4Terminal => launch_xfce4_terminal(command, title, &self.config),
            TerminalType::Xterm => launch_xterm(command, title, &self.config),
            TerminalType::RxvtUnicode => launch_urxvt(command, title, &self.config),
            TerminalType::St => launch_st(command, title, &self.config),
            TerminalType::Alacritty => launch_alacritty(command, title, &self.config),
            TerminalType::Kitty => launch_kitty(command, title, &self.config),
            TerminalType::WezTerm => launch_wezterm(command, title, &self.config),
            TerminalType::Foot => launch_foot(command, title, &self.config),
            TerminalType::Termite => launch_termite(command, title, &self.config),
            TerminalType::LilyTerm => launch_lilyterm(command, title, &self.config),
            TerminalType::Sakura => launch_sakura(command, title, &self.config),
            // macOS
            TerminalType::TerminalApp => launch_terminal_app(command, title, &self.config),
            TerminalType::ITerm2 => launch_iterm2(command, title, &self.config),
            TerminalType::Warp => launch_warp(command, title, &self.config),
            TerminalType::WezTermMac => launch_wezterm_mac(command, title, &self.config),
            TerminalType::AlacrittyMac => launch_alacritty_mac(command, title, &self.config),
            TerminalType::KittyMac => launch_kitty_mac(command, title, &self.config),
            TerminalType::Ghostty => launch_ghostty(command, title, &self.config),
            TerminalType::HyperMac => launch_hyper_mac(command, title, &self.config),
            TerminalType::TabbyMac => launch_tabby_mac(command, title, &self.config),
        };

        result.map_err(|e| TerminalError::LaunchFailed {
            terminal,
            context: context.to_string(),
            source: e,
        })
    }

    /// Launch terminal with a custom command
    pub fn launch_with_command(
        &mut self,
        command: &str,
        title: Option<&str>,
    ) -> Result<(), TerminalError> {
        let terminal = self.get_best_terminal()?;
        self.launch_with_command_ex(command, title, terminal, "custom command")
    }
}

/// Enhanced terminal error types with user-friendly messages
#[derive(Debug)]
pub enum TerminalError {
    NoTerminalFound {
        platform: Platform,
        suggestions: Vec<(&'static str, &'static str)>,
    },
    PreferredNotFound {
        terminal: TerminalType,
        install_help: Option<&'static str>,
    },
    LaunchFailed {
        terminal: TerminalType,
        context: String,
        source: LiteError,
    },
    InvalidConfiguration {
        message: String,
    },
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::NoTerminalFound {
                platform,
                suggestions,
            } => {
                writeln!(f, "No terminal emulator found on {:?}", platform)?;
                writeln!(f, "\nRecommended terminals:")?;
                for (name, help) in suggestions {
                    writeln!(f, "  - {}: {}", name, help)?;
                }
                Ok(())
            }
            TerminalError::PreferredNotFound {
                terminal,
                install_help,
            } => {
                writeln!(
                    f,
                    "Preferred terminal '{}' not found",
                    terminal.display_name()
                )?;
                if let Some(help) = install_help {
                    writeln!(f, "\nInstallation: {}", help)?;
                }
                Ok(())
            }
            TerminalError::LaunchFailed {
                terminal,
                context,
                source,
            } => {
                write!(
                    f,
                    "Failed to launch {} for '{}': {}",
                    terminal.display_name(),
                    context,
                    source
                )
            }
            TerminalError::InvalidConfiguration { message } => {
                write!(f, "Terminal configuration error: {}", message)
            }
        }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TerminalError::LaunchFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<TerminalError> for LiteError {
    fn from(e: TerminalError) -> Self {
        LiteError::Terminal(e.to_string())
    }
}

/// Generate SSH command string based on server configuration
pub fn generate_ssh_command(server: &Server) -> String {
    let mut cmd_parts = vec!["ssh".to_string()];

    // Add port if non-default
    if server.port != 22 {
        cmd_parts.push(format!("-p {}", server.port));
    }

    // Add authentication options
    match &server.auth_method {
        AuthMethod::Password { .. } => {
            cmd_parts.push("-o PreferredAuthentications=password".to_string());
            cmd_parts.push("-o PubkeyAuthentication=no".to_string());
        }
        AuthMethod::PrivateKey { key_path, .. } => {
            cmd_parts.push(format!("-i {}", escape_shell_arg(key_path)));
        }
        AuthMethod::Agent => {
            // Default SSH agent behavior, no extra args needed
        }
    }

    // Add common SSH options for better experience
    cmd_parts.push("-o StrictHostKeyChecking=accept-new".to_string());
    cmd_parts.push("-o ServerAliveInterval=60".to_string());
    cmd_parts.push("-o ServerAliveCountMax=3".to_string());

    // Add user@host
    cmd_parts.push(format!("{}@{}", server.username, server.host));

    cmd_parts.join(" ")
}

/// Escape shell argument for safe command execution
fn escape_shell_arg(arg: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows: handle spaces and special characters
        if arg.contains(' ') || arg.contains('\\') || arg.contains('"') || arg.contains('&') {
            format!("\"{}\"", arg.replace('"', "\"\""))
        } else {
            arg.to_string()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: single quote and escape
        if arg.contains('\'') {
            format!("'{}'", arg.replace('\'', "'\"'\"'"))
        } else if arg.contains(' ') || arg.contains(';') || arg.contains('&') || arg.contains('|') {
            format!("'{}'", arg)
        } else {
            arg.to_string()
        }
    }
}

/// Find executable with advanced platform-specific search
fn find_executable_advanced(name: &str, terminal_type: TerminalType) -> Option<PathBuf> {
    // Try standard PATH search first
    if let Some(path) = find_in_path(name) {
        return Some(path);
    }

    // Platform-specific extended search
    #[cfg(target_os = "windows")]
    {
        find_windows_terminal(terminal_type)
    }

    #[cfg(target_os = "macos")]
    {
        find_macos_app(terminal_type)
    }

    #[cfg(target_os = "linux")]
    {
        find_linux_desktop_entry(terminal_type)
    }
}

/// Find executable in PATH
fn find_in_path(name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Try where command
        if let Ok(output) = Command::new("where").arg(name).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return stdout.lines().next().map(|s| PathBuf::from(s.trim()));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Try which command
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Some(PathBuf::from(stdout.trim()));
            }
        }
    }

    None
}

/// Find Windows terminal with extended search
#[cfg(target_os = "windows")]
fn find_windows_terminal(terminal_type: TerminalType) -> Option<PathBuf> {
    use std::env;

    let common_paths: Vec<PathBuf> = match terminal_type {
        TerminalType::WindowsTerminal => vec![
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Microsoft")
                        .join("WindowsApps")
                        .join("wt.exe")
                })
                .ok()?,
            env::var("ProgramFiles")
                .map(|p| PathBuf::from(p).join("WindowsApps"))
                .ok()
                .and_then(|p| find_in_directory(&p, "Microsoft.WindowsTerminal_*", "wt.exe"))
                .unwrap_or_default(),
        ],
        TerminalType::PowerShell => vec![
            PathBuf::from("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"),
            env::var("SystemRoot")
                .map(|p| {
                    PathBuf::from(p)
                        .join("System32")
                        .join("WindowsPowerShell")
                        .join("v1.0")
                        .join("powershell.exe")
                })
                .ok()?,
        ],
        TerminalType::PowerShellCore => vec![
            env::var("ProgramFiles")
                .map(|p| {
                    PathBuf::from(p)
                        .join("PowerShell")
                        .join("7")
                        .join("pwsh.exe")
                })
                .ok()?,
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Microsoft")
                        .join("WindowsApps")
                        .join("pwsh.exe")
                })
                .ok()?,
            PathBuf::from("C:\\Program Files\\PowerShell\\7\\pwsh.exe"),
        ],
        TerminalType::GitBash => vec![
            env::var("ProgramFiles")
                .map(|p| PathBuf::from(p).join("Git").join("git-bash.exe"))
                .ok()?,
            env::var("ProgramFiles(x86)")
                .map(|p| PathBuf::from(p).join("Git").join("git-bash.exe"))
                .ok()?,
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Programs")
                        .join("Git")
                        .join("git-bash.exe")
                })
                .ok()?,
            env::var("USERPROFILE")
                .map(|p| {
                    PathBuf::from(p)
                        .join("scoop")
                        .join("apps")
                        .join("git")
                        .join("current")
                        .join("git-bash.exe")
                })
                .ok()?,
        ],
        TerminalType::Hyper => vec![
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Programs")
                        .join("hyper")
                        .join("Hyper.exe")
                })
                .ok()?,
            env::var("APPDATA")
                .map(|p| PathBuf::from(p).join("Hyper").join("Hyper.exe"))
                .ok()?,
        ],
        TerminalType::Tabby => vec![
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Programs")
                        .join("Tabby")
                        .join("Tabby.exe")
                })
                .ok()?,
            env::var("ProgramFiles")
                .map(|p| PathBuf::from(p).join("Tabby").join("Tabby.exe"))
                .ok()?,
        ],
        TerminalType::AlacrittyWindows => vec![
            env::var("ProgramFiles")
                .map(|p| PathBuf::from(p).join("Alacritty").join("alacritty.exe"))
                .ok()?,
            env::var("USERPROFILE")
                .map(|p| {
                    PathBuf::from(p)
                        .join("scoop")
                        .join("apps")
                        .join("alacritty")
                        .join("current")
                        .join("Alacritty.exe")
                })
                .ok()?,
            env::var("LOCALAPPDATA")
                .map(|p| {
                    PathBuf::from(p)
                        .join("Microsoft")
                        .join("WindowsApps")
                        .join("alacritty.exe")
                })
                .ok()?,
        ],
        TerminalType::WezTermWindows => vec![
            env::var("ProgramFiles")
                .map(|p| PathBuf::from(p).join("WezTerm").join("wezterm.exe"))
                .ok()?,
            env::var("USERPROFILE")
                .map(|p| {
                    PathBuf::from(p)
                        .join("scoop")
                        .join("apps")
                        .join("wezterm")
                        .join("current")
                        .join("wezterm.exe")
                })
                .ok()?,
        ],
        _ => return None,
    };

    common_paths.into_iter().find(|path| path.exists())
}

/// Find in directory with pattern
#[cfg(target_os = "windows")]
fn find_in_directory(dir: &Path, pattern: &str, file: &str) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(pattern.trim_end_matches("*")) {
                let full_path = entry.path().join(file);
                if full_path.exists() {
                    return Some(full_path);
                }
            }
        }
    }
    None
}

/// Find macOS app bundles
#[cfg(target_os = "macos")]
fn find_macos_app(terminal_type: TerminalType) -> Option<PathBuf> {
    let app_names: Vec<&str> = match terminal_type {
        TerminalType::TerminalApp => vec!["Terminal"],
        TerminalType::ITerm2 => vec!["iTerm", "iTerm2", "iTerm.app"],
        TerminalType::Warp => vec!["Warp"],
        TerminalType::WezTermMac => vec!["WezTerm"],
        TerminalType::AlacrittyMac => vec!["Alacritty"],
        TerminalType::KittyMac => vec!["kitty"],
        TerminalType::Ghostty => vec!["Ghostty"],
        TerminalType::HyperMac => vec!["Hyper"],
        TerminalType::TabbyMac => vec!["Tabby"],
        _ => return None,
    };

    let search_paths = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        dirs::home_dir()
            .map(|p| p.join("Applications"))
            .unwrap_or_default(),
    ];

    for app_name in app_names {
        for base_path in &search_paths {
            let app_path = base_path.join(format!("{}.app", app_name));
            let executable_path = app_path.join("Contents").join("MacOS").join(app_name);
            if executable_path.exists() {
                return Some(executable_path);
            }
        }
    }

    None
}

/// Find Linux desktop entries
#[cfg(target_os = "linux")]
fn find_linux_desktop_entry(terminal_type: TerminalType) -> Option<PathBuf> {
    let desktop_names: Vec<&str> = match terminal_type {
        TerminalType::GnomeTerminal => vec!["gnome-terminal.desktop"],
        TerminalType::Konsole => vec!["org.kde.konsole.desktop"],
        TerminalType::Yakuake => vec!["org.kde.yakuake.desktop"],
        TerminalType::Xfce4Terminal => vec!["xfce4-terminal.desktop"],
        TerminalType::Tilix => vec!["com.gexperts.Tilix.desktop"],
        TerminalType::Terminator => vec!["terminator.desktop"],
        TerminalType::Alacritty => vec!["Alacritty.desktop", "alacritty.desktop"],
        TerminalType::Kitty => vec!["kitty.desktop"],
        TerminalType::WezTerm => vec!["org.wezfurlong.wezterm.desktop"],
        _ => return None,
    };

    let data_dirs = vec![
        dirs::data_dir(),
        Some(PathBuf::from("/usr/share/applications")),
        Some(PathBuf::from("/usr/local/share/applications")),
        Some(PathBuf::from("/var/lib/flatpak/exports/share/applications")),
    ];

    for desktop_name in desktop_names {
        for dir in data_dirs.iter().flatten() {
            let desktop_path = dir.join(desktop_name);
            if desktop_path.exists() {
                // Extract Exec line from desktop file
                if let Ok(content) = std::fs::read_to_string(&desktop_path) {
                    for line in content.lines() {
                        if line.starts_with("Exec=") {
                            let exec =
                                line.trim_start_matches("Exec=").split_whitespace().next()?;
                            return find_in_path(exec);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Try to get terminal version
fn try_get_version(path: &Path, terminal_type: TerminalType) -> Option<String> {
    let version_arg = match terminal_type {
        TerminalType::Alacritty | TerminalType::AlacrittyMac | TerminalType::AlacrittyWindows => {
            "--version"
        }
        TerminalType::WezTerm | TerminalType::WezTermMac | TerminalType::WezTermWindows => {
            "--version"
        }
        TerminalType::PowerShellCore => "--version",
        _ => return None,
    };

    if let Ok(output) = Command::new(path).arg(version_arg).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Some(stdout.trim().to_string());
        }
    }

    None
}

// ============================================================================
// Windows Terminal Launchers
// ============================================================================

#[cfg(target_os = "windows")]
fn launch_windows_terminal(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let in_wt = std::env::var("WT_SESSION").is_ok();

    let wt_path = find_executable_advanced("wt", TerminalType::WindowsTerminal)
        .or_else(|| find_executable_advanced("wt.exe", TerminalType::WindowsTerminal))
        .ok_or_else(|| LiteError::Terminal("Windows Terminal executable not found".to_string()))?;

    let mut cmd = Command::new(&wt_path);

    // Add custom args if any
    if let Some(args) = config.custom_args.get(&TerminalType::WindowsTerminal) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    if in_wt {
        // Already in Windows Terminal, open new tab
        cmd.arg("new-tab");
        if let Some(t) = title {
            cmd.arg("--title").arg(t);
        }
    } else {
        // New window
        if let Some(t) = title {
            cmd.arg("--title").arg(t);
        }
    }

    // Use PowerShell as the host for better SSH experience
    cmd.arg("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Windows Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_powershell(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Command::new("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch PowerShell: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_pwsh(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("pwsh.exe");

    // Add custom args
    if let Some(args) = config.custom_args.get(&TerminalType::PowerShellCore) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("-NoExit").arg("-Command").arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch PowerShell 7: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_cmd(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("cmd.exe");

    // Add custom args
    if let Some(args) = config.custom_args.get(&TerminalType::Cmd) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("/K").arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch CMD: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_gitbash(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let git_bash_path = find_windows_terminal(TerminalType::GitBash).ok_or_else(|| {
        LiteError::Terminal(
            "Git Bash not found. Please install Git for Windows: https://git-scm.com/download/win"
                .to_string(),
        )
    })?;

    Command::new(&git_bash_path)
        .arg("-c")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Git Bash: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_fluent_terminal(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Command::new("FluentTerminal")
        .arg("run")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Fluent Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_hyper(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let hyper_path = find_windows_terminal(TerminalType::Hyper).ok_or_else(|| {
        LiteError::Terminal("Hyper not found. Please install from https://hyper.is".to_string())
    })?;

    Command::new(&hyper_path)
        .arg("-e")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Hyper: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_tabby(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let tabby_path = find_windows_terminal(TerminalType::Tabby).ok_or_else(|| {
        LiteError::Terminal("Tabby not found. Please install from https://tabby.sh".to_string())
    })?;

    Command::new(&tabby_path)
        .arg("run")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Tabby: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_alacritty_windows(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let alacritty_path =
        find_windows_terminal(TerminalType::AlacrittyWindows).ok_or_else(|| {
            LiteError::Terminal(
                "Alacritty not found. Please install from https://alacritty.org".to_string(),
            )
        })?;

    let mut cmd = Command::new(&alacritty_path);

    // Add custom args
    if let Some(args) = config.custom_args.get(&TerminalType::AlacrittyWindows) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("-e")
        .arg("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Alacritty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_wezterm_windows(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let wezterm_path = find_windows_terminal(TerminalType::WezTermWindows).ok_or_else(|| {
        LiteError::Terminal(
            "WezTerm not found. Please install from https://wezfurlong.org/wezterm".to_string(),
        )
    })?;

    let mut cmd = Command::new(&wezterm_path);

    // Add custom args
    if let Some(args) = config.custom_args.get(&TerminalType::WezTermWindows) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("start")
        .arg("--")
        .arg("powershell.exe")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch WezTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_cygwin(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Command::new("mintty")
        .arg("-e")
        .arg("/bin/bash")
        .arg("-lc")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Cygwin: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_msys2(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let msys2_path = PathBuf::from("C:\\msys64\\msys2.exe");
    if !msys2_path.exists() {
        return Err(LiteError::Terminal(
            "MSYS2 not found at C:\\msys64\\msys2.exe".to_string(),
        ));
    }

    Command::new(&msys2_path)
        .arg("-c")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch MSYS2: {}", e)))?;

    Ok(())
}

// ============================================================================
// Linux Terminal Launchers
// ============================================================================

#[cfg(target_os = "linux")]
fn launch_gnome_terminal(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("gnome-terminal");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    // Add custom args
    if let Some(args) = config.custom_args.get(&TerminalType::GnomeTerminal) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("--")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch GNOME Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_tilix(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("tilix");

    if let Some(t) = title {
        cmd.arg("-t").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Tilix) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Tilix: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_terminator(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("terminator");

    if let Some(t) = title {
        cmd.arg("-T").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Terminator) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e").arg(format!(
        "bash -c \"{}{}\"",
        command.replace('"', "\\\""),
        wait_suffix
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Terminator: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_konsole(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("konsole");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Konsole) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Konsole: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_yakuake(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    // First, ensure Yakuake is running
    let _ = Command::new("qdbus")
        .args(["org.kde.yakuake", "/yakuake/window", "isOpen"])
        .output();

    let mut cmd = Command::new("konsole");

    if let Some(args) = config.custom_args.get(&TerminalType::Yakuake) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Yakuake session: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_xfce4_terminal(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("xfce4-terminal");

    if let Some(t) = title {
        cmd.arg("--title").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Xfce4Terminal) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e").arg(format!(
        "bash -c \"{}{}\"",
        command.replace('"', "\\\""),
        wait_suffix
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch XFCE4 Terminal: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_xterm(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("xterm");

    if let Some(t) = title {
        cmd.arg("-title").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Xterm) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch XTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_urxvt(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("urxvt");

    if let Some(t) = title {
        cmd.arg("-title").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::RxvtUnicode) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch RXVT-Unicode: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_st(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("st");

    if let Some(args) = config.custom_args.get(&TerminalType::St) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch st: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_alacritty(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("alacritty");

    if let Some(args) = config.custom_args.get(&TerminalType::Alacritty) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Alacritty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_kitty(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("kitty");

    if let Some(args) = config.custom_args.get(&TerminalType::Kitty) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Kitty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_wezterm(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("wezterm");

    if let Some(args) = config.custom_args.get(&TerminalType::WezTerm) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("start")
        .arg("--")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch WezTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_foot(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("foot");

    if let Some(args) = config.custom_args.get(&TerminalType::Foot) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Foot: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_termite(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("termite");

    if let Some(args) = config.custom_args.get(&TerminalType::Termite) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e").arg(format!(
        "bash -c \"{}{}\"",
        command.replace('"', "\\\""),
        wait_suffix
    ));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Termite: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_lilyterm(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("lilyterm");

    if let Some(t) = title {
        cmd.arg("-t").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::LilyTerm) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch LilyTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_sakura(
    command: &str,
    title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let mut cmd = Command::new("sakura");

    if let Some(t) = title {
        cmd.arg("-t").arg(t);
    }

    if let Some(args) = config.custom_args.get(&TerminalType::Sakura) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let wait_suffix = if config.keep_open {
        "; read -p 'Press Enter to exit...'"
    } else {
        ""
    };

    cmd.arg("-e")
        .arg("bash")
        .arg("-c")
        .arg(format!("{}{}", command, wait_suffix));

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Sakura: {}", e)))?;

    Ok(())
}

// ============================================================================
// macOS Terminal Launchers
// ============================================================================

#[cfg(target_os = "macos")]
fn launch_terminal_app(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let escaped_cmd = command.replace('"', "\\\"");
    let script = format!(
        r#"tell application "Terminal"
            if not running then launch
            activate
            do script "{}"
        end tell"#,
        escaped_cmd
    );

    // Add custom AppleScript if provided
    let final_script = if let Some(args) = config.custom_args.get(&TerminalType::TerminalApp) {
        if let Some(applescript) = args.first() {
            format!("{}\n{}", script, applescript)
        } else {
            script
        }
    } else {
        script
    };

    Command::new("osascript")
        .arg("-e")
        .arg(&final_script)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Terminal.app: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_iterm2(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let escaped_cmd = command.replace('"', "\\\"");
    let script = format!(
        r#"tell application "iTerm"
            if not running then launch
            activate
            tell current window
                create tab with default profile
                tell current session
                    write text "{}"
                end tell
            end tell
        end tell"#,
        escaped_cmd
    );

    let final_script = if let Some(args) = config.custom_args.get(&TerminalType::ITerm2) {
        if let Some(applescript) = args.first() {
            format!("{}\n{}", script, applescript)
        } else {
            script
        }
    } else {
        script
    };

    Command::new("osascript")
        .arg("-e")
        .arg(&final_script)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch iTerm2: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_warp(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let warp_path = find_macos_app(TerminalType::Warp)
        .or_else(|| Some(PathBuf::from("/Applications/Warp.app")))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            LiteError::Terminal("Warp not found. Please install from https://warp.dev".to_string())
        })?;

    // Warp has limited CLI support, use open command
    Command::new("open")
        .arg("-a")
        .arg("Warp")
        .arg("--args")
        .arg("-e")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Warp: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_wezterm_mac(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let wezterm_path = find_macos_app(TerminalType::WezTermMac)
        .or_else(|| find_in_path("wezterm"))
        .ok_or_else(|| {
            LiteError::Terminal(
                "WezTerm not found. Please install from https://wezfurlong.org/wezterm".to_string(),
            )
        })?;

    let mut cmd = Command::new(&wezterm_path);

    if let Some(args) = config.custom_args.get(&TerminalType::WezTermMac) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("cli")
        .arg("spawn")
        .arg("--")
        .arg("bash")
        .arg("-c")
        .arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch WezTerm: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_alacritty_mac(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let alacritty_path = find_macos_app(TerminalType::AlacrittyMac)
        .or_else(|| find_in_path("alacritty"))
        .ok_or_else(|| {
            LiteError::Terminal(
                "Alacritty not found. Please install from https://alacritty.org".to_string(),
            )
        })?;

    let mut cmd = Command::new(&alacritty_path);

    if let Some(args) = config.custom_args.get(&TerminalType::AlacrittyMac) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("-e").arg("bash").arg("-c").arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Alacritty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_kitty_mac(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let kitty_path = find_macos_app(TerminalType::KittyMac)
        .or_else(|| find_in_path("kitty"))
        .ok_or_else(|| {
            LiteError::Terminal(
                "Kitty not found. Please install: https://sw.kovidgoyal.net/kitty".to_string(),
            )
        })?;

    let mut cmd = Command::new(&kitty_path);

    if let Some(args) = config.custom_args.get(&TerminalType::KittyMac) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("-e").arg("bash").arg("-c").arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Kitty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_ghostty(
    command: &str,
    _title: Option<&str>,
    config: &TerminalConfig,
) -> Result<(), LiteError> {
    let ghostty_path = find_macos_app(TerminalType::Ghostty)
        .or_else(|| Some(PathBuf::from("/Applications/Ghostty.app")))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            LiteError::Terminal(
                "Ghostty not found. Please install from https://mitchellh.com/ghostty".to_string(),
            )
        })?;

    let mut cmd = Command::new(&ghostty_path.join("Contents").join("MacOS").join("ghostty"));

    if let Some(args) = config.custom_args.get(&TerminalType::Ghostty) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    cmd.arg("-e").arg("bash").arg("-c").arg(command);

    cmd.spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Ghostty: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_hyper_mac(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let hyper_path = find_macos_app(TerminalType::HyperMac)
        .or_else(|| Some(PathBuf::from("/Applications/Hyper.app")))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            LiteError::Terminal("Hyper not found. Please install from https://hyper.is".to_string())
        })?;

    Command::new("open")
        .arg("-a")
        .arg("Hyper")
        .arg("--args")
        .arg("-e")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Hyper: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_tabby_mac(
    command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    let tabby_path = find_macos_app(TerminalType::TabbyMac)
        .or_else(|| Some(PathBuf::from("/Applications/Tabby.app")))
        .filter(|p| p.exists())
        .ok_or_else(|| {
            LiteError::Terminal("Tabby not found. Please install from https://tabby.sh".to_string())
        })?;

    Command::new("open")
        .arg("-a")
        .arg("Tabby")
        .arg("--args")
        .arg(command)
        .spawn()
        .map_err(|e| LiteError::Terminal(format!("Failed to launch Tabby: {}", e)))?;

    Ok(())
}

// ============================================================================
// Stub implementations for non-target platforms (for compilation)
// ============================================================================

#[cfg(not(target_os = "windows"))]
fn launch_windows_terminal(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Windows Terminal not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_powershell(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "PowerShell not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_pwsh(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "PowerShell 7 not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_cmd(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "CMD not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_gitbash(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Git Bash not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_fluent_terminal(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Fluent Terminal not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_hyper(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Hyper not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_tabby(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Tabby not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_alacritty_windows(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Alacritty Windows build not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_wezterm_windows(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "WezTerm Windows build not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_cygwin(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Cygwin not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
fn launch_msys2(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "MSYS2 not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_gnome_terminal(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "GNOME Terminal not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_tilix(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Tilix not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_terminator(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Terminator not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_konsole(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Konsole not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_yakuake(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Yakuake not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_xfce4_terminal(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "XFCE4 Terminal not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_xterm(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "XTerm not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_urxvt(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "RXVT-Unicode not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_st(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "st not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_alacritty(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Alacritty not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_kitty(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Kitty not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_wezterm(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "WezTerm not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_foot(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Foot not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_termite(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Termite not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_lilyterm(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "LilyTerm not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "linux"))]
fn launch_sakura(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Sakura not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_terminal_app(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Terminal.app not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_iterm2(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "iTerm2 not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_warp(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Warp not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_wezterm_mac(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "WezTerm not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_alacritty_mac(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Alacritty not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_kitty_mac(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Kitty not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_ghostty(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Ghostty not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_hyper_mac(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Hyper not available on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
fn launch_tabby_mac(
    _command: &str,
    _title: Option<&str>,
    _config: &TerminalConfig,
) -> Result<(), LiteError> {
    Err(LiteError::Terminal(
        "Tabby not available on this platform".to_string(),
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::server::{AuthMethod, Server};

    fn create_test_server(auth_method: AuthMethod) -> Server {
        Server::new(
            "Test Server".to_string(),
            "192.168.1.1".to_string(),
            22,
            "admin".to_string(),
            auth_method,
            None,
        )
    }

    #[test]
    fn test_generate_ssh_command_agent() {
        let server = create_test_server(AuthMethod::Agent);
        let cmd = generate_ssh_command(&server);
        assert!(cmd.contains("ssh"));
        assert!(cmd.contains("admin@192.168.1.1"));
        assert!(cmd.contains("ServerAliveInterval=60"));
    }

    #[test]
    fn test_generate_ssh_command_password() {
        let server = create_test_server(AuthMethod::Password {
            password: "secret".to_string(),
        });
        let cmd = generate_ssh_command(&server);
        assert!(cmd.contains("PreferredAuthentications=password"));
        assert!(cmd.contains("PubkeyAuthentication=no"));
    }

    #[test]
    fn test_generate_ssh_command_key() {
        let server = create_test_server(AuthMethod::PrivateKey {
            key_path: "/home/user/.ssh/id_rsa".to_string(),
            passphrase: None,
        });
        let cmd = generate_ssh_command(&server);
        assert!(cmd.contains("-i"));
        assert!(cmd.contains("/home/user/.ssh/id_rsa"));
    }

    #[test]
    fn test_terminal_type_display_name() {
        assert_eq!(
            TerminalType::WindowsTerminal.display_name(),
            "Windows Terminal"
        );
        assert_eq!(TerminalType::PowerShellCore.display_name(), "PowerShell 7+");
        assert_eq!(TerminalType::GnomeTerminal.display_name(), "GNOME Terminal");
        assert_eq!(TerminalType::Warp.display_name(), "Warp");
    }

    #[test]
    fn test_terminal_type_platform() {
        assert_eq!(TerminalType::WindowsTerminal.platform(), Platform::Windows);
        assert_eq!(TerminalType::GnomeTerminal.platform(), Platform::Linux);
        assert_eq!(TerminalType::ITerm2.platform(), Platform::MacOS);
    }

    #[test]
    fn test_terminal_type_install_help() {
        assert!(TerminalType::Warp.install_help().is_some());
        assert!(TerminalType::WindowsTerminal.install_help().is_some());
        assert!(TerminalType::Cmd.install_help().is_none());
    }

    #[test]
    fn test_terminal_platform_terminals() {
        let terminals = TerminalType::platform_terminals();
        assert!(!terminals.is_empty());

        #[cfg(target_os = "windows")]
        {
            assert!(terminals.contains(&TerminalType::WindowsTerminal));
            assert!(terminals.contains(&TerminalType::PowerShellCore));
            assert!(terminals.contains(&TerminalType::Cmd));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(terminals.contains(&TerminalType::GnomeTerminal));
            assert!(terminals.contains(&TerminalType::Konsole));
            assert!(terminals.contains(&TerminalType::Tilix));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(terminals.contains(&TerminalType::TerminalApp));
            assert!(terminals.contains(&TerminalType::ITerm2));
            assert!(terminals.contains(&TerminalType::Warp));
        }
    }

    #[test]
    fn test_terminal_preference_default() {
        let pref = TerminalPreference::default();
        assert!(matches!(pref, TerminalPreference::Auto));
    }

    #[test]
    fn test_terminal_launcher_builder() {
        let launcher = TerminalLauncher::with_preference(TerminalPreference::Specific(
            TerminalType::Alacritty,
        ));

        assert!(matches!(
            launcher.preference,
            TerminalPreference::Specific(TerminalType::Alacritty)
        ));
    }

    #[test]
    fn test_escape_shell_arg() {
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(escape_shell_arg("simple"), "simple");
            assert_eq!(escape_shell_arg("with space"), "'with space'");
            assert_eq!(escape_shell_arg("path'quote"), "'path'\"'\"'quote'");
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(escape_shell_arg("simple"), "simple");
            assert_eq!(escape_shell_arg("with space"), "\"with space\"");
            assert_eq!(escape_shell_arg("path\"quote"), "\"path\"\"quote\"");
        }
    }

    #[test]
    fn test_terminal_error_display() {
        let err = TerminalError::NoTerminalFound {
            platform: Platform::Windows,
            suggestions: vec![("Windows Terminal", "Microsoft Store")],
        };
        let msg = format!("{}", err);
        assert!(msg.contains("No terminal emulator found"));
        assert!(msg.contains("Windows Terminal"));
    }

    #[test]
    fn test_terminal_config_default() {
        let config = TerminalConfig::default();
        assert!(config.custom_args.is_empty());
        assert!(config.env_vars.is_empty());
        assert!(config.working_dir.is_none());
        assert!(config.keep_open);
    }

    #[test]
    fn test_terminal_all_terminals() {
        let all = TerminalType::all_terminals();
        assert!(all.len() >= 30); // Should have many terminals
        assert!(all.contains(&TerminalType::WindowsTerminal));
        assert!(all.contains(&TerminalType::Alacritty));
        assert!(all.contains(&TerminalType::ITerm2));
    }
}
