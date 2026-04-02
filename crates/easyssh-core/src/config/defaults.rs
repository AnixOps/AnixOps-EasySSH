//! Configuration Defaults
//!
//! Provides default values for all configuration settings
//! and utilities for working with defaults.

use super::types::*;

/// Get default application configuration
pub fn default_app_config() -> AppConfig {
    AppConfig::default()
}

/// Get default user preferences
pub fn default_user_preferences() -> UserPreferences {
    UserPreferences::default()
}

/// Get default security settings
pub fn default_security_settings() -> SecuritySettings {
    SecuritySettings::default()
}

/// Get default full configuration
pub fn default_full_config() -> FullConfig {
    FullConfig::default()
}

/// Configuration presets for different user types
pub struct ConfigPresets;

impl ConfigPresets {
    /// Minimal configuration for power users
    pub fn minimal() -> FullConfig {
        let mut config = FullConfig::default();

        // Minimal UI
        config.app_config.show_sidebar = false;
        config.app_config.restore_window_geometry = false;

        // No history
        config.user_preferences.max_search_history = 0;
        config.user_preferences.max_recent_connections = 0;
        config.user_preferences.show_notifications = false;
        config.user_preferences.sound_effects = false;

        // High security
        config.security_settings.clipboard_clear_time = 10;
        config.security_settings.lock_on_sleep = true;
        config.security_settings.lock_on_blur = true;

        config
    }

    /// Balanced configuration (default)
    pub fn balanced() -> FullConfig {
        FullConfig::default()
    }

    /// Feature-rich configuration for beginners
    pub fn rich() -> FullConfig {
        let mut config = FullConfig::default();

        // Full UI
        config.app_config.show_sidebar = true;
        config.app_config.restore_window_geometry = true;
        config.app_config.sidebar_width = 300;

        // Extensive history
        config.user_preferences.max_search_history = 200;
        config.user_preferences.max_recent_connections = 50;
        config.user_preferences.show_notifications = true;
        config.user_preferences.sound_effects = true;
        config.user_preferences.confirm_close_active = true;

        // Moderate security
        config.security_settings.clipboard_clear_time = 60;
        config.security_settings.lock_on_sleep = true;
        config.security_settings.audit_sensitive_ops = true;

        config
    }

    /// Enterprise configuration for corporate environments
    pub fn enterprise() -> FullConfig {
        let mut config = FullConfig::default();

        // Secure defaults
        config.security_settings.require_password_on_startup = true;
        config.security_settings.master_password_timeout = 30;
        config.security_settings.auto_lock_after_idle = 15;
        config.security_settings.lock_on_sleep = true;
        config.security_settings.lock_on_blur = true;
        config.security_settings.clipboard_clear_time = 15;
        config.security_settings.strict_host_key_checking = true;
        config.security_settings.audit_sensitive_ops = true;
        config.security_settings.encrypt_backups = true;
        config.security_settings.min_password_length = 12;
        config.security_settings.require_password_complexity = true;

        // No auto-save for compliance
        config.user_preferences.auto_save_connections = false;
        config.user_preferences.show_notifications = false;

        config
    }
}

/// Get system-appropriate default terminal
pub fn get_system_default_terminal() -> String {
    #[cfg(target_os = "windows")]
    {
        // Try to find the best available terminal on Windows
        if std::path::Path::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe").exists() {
            "powershell.exe".to_string()
        } else if std::path::Path::new("C:\\Windows\\System32\\cmd.exe").exists() {
            "cmd.exe".to_string()
        } else {
            "powershell.exe".to_string()
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Check for common terminal apps on macOS
        let terminals = [
            "/Applications/iTerm.app",
            "/System/Applications/Terminal.app",
            "/Applications/Terminal.app",
        ];

        for terminal in &terminals {
            if std::path::Path::new(terminal).exists() {
                return std::path::Path::new(terminal)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Terminal.app")
                    .to_string();
            }
        }

        "Terminal.app".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        // Common terminal emulators on Linux in order of preference
        let terminals = [
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "lxterminal",
            "st",
            "alacritty",
            "kitty",
            "xterm",
        ];

        // Return the first one that might be available
        // In a real implementation, we'd check PATH
        for terminal in &terminals {
            return terminal.to_string();
        }

        // Fallback
        "xterm".to_string()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "terminal".to_string()
    }
}

/// Detect system theme preference
pub fn detect_system_theme() -> Theme {
    // This would ideally query the OS for the current theme
    // For now, default to System which lets the app follow OS preference
    Theme::System
}

/// Detect system language
pub fn detect_system_language() -> Language {
    // Try to get system locale
    if let Some(locale) = sys_locale::get_locale() {
        let locale_str = locale.to_string().to_lowercase();
        if locale_str.starts_with("zh") {
            return Language::Chinese;
        }
    }

    // Check environment variable
    if let Ok(lang) = std::env::var("LANG") {
        let lang_lower = lang.to_lowercase();
        if lang_lower.contains("zh") {
            return Language::Chinese;
        }
    }

    Language::English
}

/// Environment-based configuration overrides
pub struct EnvConfig;

impl EnvConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Option<FullConfig> {
        let mut config = FullConfig::default();
        let mut modified = false;

        // Theme override
        if let Ok(theme) = std::env::var("EASYSSH_THEME") {
            config.app_config.theme = match theme.to_lowercase().as_str() {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                "system" => Theme::System,
                _ => config.app_config.theme,
            };
            modified = true;
        }

        // Language override
        if let Ok(lang) = std::env::var("EASYSSH_LANG") {
            config.app_config.language = match lang.to_lowercase().as_str() {
                "zh" | "zh-cn" | "zh-tw" => Language::Chinese,
                "en" => Language::English,
                _ => config.app_config.language,
            };
            modified = true;
        }

        // Terminal override
        if let Ok(term) = std::env::var("EASYSSH_TERMINAL") {
            config.app_config.default_terminal = term;
            modified = true;
        }

        // Port override
        if let Ok(port) = std::env::var("EASYSSH_DEFAULT_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                if port_num > 0 {
                    config.user_preferences.default_port = port_num;
                    modified = true;
                }
            }
        }

        // Username override
        if let Ok(user) = std::env::var("EASYSSH_DEFAULT_USER") {
            config.user_preferences.default_username = user;
            modified = true;
        }

        if modified {
            Some(config)
        } else {
            None
        }
    }

    /// Apply environment overrides to existing config
    pub fn apply_overrides(config: &mut FullConfig) {
        if let Some(env_config) = Self::from_env() {
            // Apply non-default values from env
            if env_config.app_config.theme != Theme::System {
                config.app_config.theme = env_config.app_config.theme;
            }
            if env_config.app_config.language != Language::English {
                config.app_config.language = env_config.app_config.language;
            }
            if !env_config.app_config.default_terminal.is_empty() {
                config.app_config.default_terminal = env_config.app_config.default_terminal;
            }
            if env_config.user_preferences.default_port != 22 {
                config.user_preferences.default_port = env_config.user_preferences.default_port;
            }
            if !env_config.user_preferences.default_username.is_empty() {
                config.user_preferences.default_username =
                    env_config.user_preferences.default_username;
            }
        }
    }
}

/// Apply system-detected defaults
pub fn apply_system_defaults(config: &mut FullConfig) {
    // Only apply if using system defaults
    if config.app_config.theme == Theme::System {
        config.app_config.theme = detect_system_theme();
    }

    // Detect language if not explicitly set
    // (We don't override user preference, only set on fresh configs)

    // Apply system-appropriate terminal if not set
    if config.app_config.default_terminal.is_empty() {
        config.app_config.default_terminal = get_system_default_terminal();
    }

    // Try to get username from system
    if config.user_preferences.default_username.is_empty() {
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            config.user_preferences.default_username = user;
        } else if let Some(user) = whoami::username().split('\\').last() {
            // Handle Windows DOMAIN\username format
            config.user_preferences.default_username = user.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_presets() {
        let minimal = ConfigPresets::minimal();
        assert!(!minimal.app_config.show_sidebar);
        assert_eq!(minimal.user_preferences.max_search_history, 0);

        let rich = ConfigPresets::rich();
        assert!(rich.app_config.show_sidebar);
        assert_eq!(rich.user_preferences.max_search_history, 200);

        let enterprise = ConfigPresets::enterprise();
        assert!(enterprise.security_settings.require_password_on_startup);
        assert!(enterprise.security_settings.require_password_complexity);
    }

    #[test]
    fn test_env_config() {
        // Set environment variables
        std::env::set_var("EASYSSH_THEME", "dark");
        std::env::set_var("EASYSSH_LANG", "zh");

        let config = EnvConfig::from_env();
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.app_config.theme, Theme::Dark);
        assert_eq!(config.app_config.language, Language::Chinese);

        // Clean up
        std::env::remove_var("EASYSSH_THEME");
        std::env::remove_var("EASYSSH_LANG");
    }

    #[test]
    fn test_apply_overrides() {
        let mut config = FullConfig::default();

        // Set environment variables
        std::env::set_var("EASYSSH_THEME", "light");

        EnvConfig::apply_overrides(&mut config);

        assert_eq!(config.app_config.theme, Theme::Light);

        // Clean up
        std::env::remove_var("EASYSSH_THEME");
    }

    #[test]
    fn test_detect_language() {
        // This test depends on system locale
        // We just verify it doesn't panic
        let _lang = detect_system_language();
    }

    #[test]
    fn test_get_system_default_terminal() {
        // Just verify it returns a non-empty string
        let term = get_system_default_terminal();
        assert!(!term.is_empty());
    }

    #[test]
    fn test_apply_system_defaults() {
        let mut config = FullConfig::default();
        config.app_config.default_terminal.clear();
        config.user_preferences.default_username.clear();

        apply_system_defaults(&mut config);

        // Should have filled in defaults
        assert!(!config.app_config.default_terminal.is_empty());
    }
}
