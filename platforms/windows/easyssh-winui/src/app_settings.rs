//! Application settings persistence for EasySSH WinUI
//!
//! Handles persistence of user preferences like:
//! - Language setting
//! - UI theme mode (dark/light/system)
//! - Accessibility settings
//!
//! Settings are stored in: %APPDATA%/easyssh/settings.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Application settings that should persist across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Selected language code (e.g., "en", "zh-CN")
    pub language: String,

    /// UI theme mode: "dark", "light", or "system"
    pub theme_mode: String,

    /// High contrast mode enabled
    pub high_contrast: bool,

    /// Reduced motion enabled
    pub reduced_motion: bool,

    /// Large text enabled
    pub large_text: bool,

    /// Terminal font family (e.g., "Cascadia Code", "JetBrains Mono")
    pub terminal_font_family: String,

    /// Terminal base font size in pixels
    pub terminal_font_size: f32,

    /// Terminal font zoom level (1.0 = 100%)
    pub terminal_font_zoom: f32,

    /// Whether to use WebGL for terminal rendering
    pub terminal_use_webgl: bool,

    /// Auto-scroll terminal output
    pub terminal_auto_scroll: bool,

    /// Copy on select in terminal
    pub terminal_copy_on_select: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme_mode: "dark".to_string(),
            high_contrast: false,
            reduced_motion: false,
            large_text: false,
            terminal_font_family: "Cascadia Code".to_string(),
            terminal_font_size: 14.0,
            terminal_font_zoom: 1.0,
            terminal_use_webgl: true,
            terminal_auto_scroll: true,
            terminal_copy_on_select: false,
        }
    }
}

impl AppSettings {
    /// Load settings from disk or return defaults
    pub fn load() -> Self {
        match Self::settings_path() {
            Some(path) => {
                if path.exists() {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match serde_json::from_str::<AppSettings>(&content) {
                            Ok(settings) => {
                                info!("Loaded settings from {:?}", path);
                                return settings;
                            }
                            Err(e) => {
                                warn!("Failed to parse settings file: {}, using defaults", e);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to read settings file: {}, using defaults", e);
                        }
                    }
                } else {
                    info!("No settings file found at {:?}, using defaults", path);
                }
            }
            None => {
                warn!("Could not determine settings path, using defaults");
            }
        }

        Self::default()
    }

    /// Save settings to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine settings path"))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        info!("Saved settings to {:?}", path);
        Ok(())
    }

    /// Get the settings file path
    fn settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("easyssh").join("settings.json"))
    }

    /// Apply language setting to the i18n system
    pub fn apply_language(&self) -> anyhow::Result<()> {
        use easyssh_core::set_language;

        if let Err(e) = set_language(&self.language) {
            warn!("Failed to set language to '{}': {}", self.language, e);
            return Err(anyhow::anyhow!("Failed to set language: {}", e));
        }

        info!("Applied language: {}", self.language);
        Ok(())
    }

    /// Load and apply settings in one operation
    pub fn load_and_apply() -> Self {
        let settings = Self::load();

        // Apply language setting
        if let Err(e) = settings.apply_language() {
            error!("Failed to apply language setting: {}", e);
        }

        settings
    }
}

/// Thread-safe settings manager that maintains both the settings
/// and a flag indicating if settings have changed and need to be saved
#[derive(Debug)]
pub struct SettingsManager {
    settings: Arc<Mutex<AppSettings>>,
    dirty: Arc<Mutex<bool>>,
}

impl SettingsManager {
    /// Create a new settings manager, loading from disk
    pub fn new() -> Self {
        let settings = AppSettings::load_and_apply();

        Self {
            settings: Arc::new(Mutex::new(settings)),
            dirty: Arc::new(Mutex::new(false)),
        }
    }

    /// Get a copy of the current settings
    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    /// Update the language setting
    pub fn set_language(&self, language: String) -> anyhow::Result<()> {
        let mut settings = self.settings.lock().unwrap();

        if settings.language != language {
            settings.language = language.clone();

            // Apply immediately
            settings.apply_language()?;

            // Mark as dirty to be saved
            *self.dirty.lock().unwrap() = true;

            info!("Language setting updated to: {}", language);
        }

        Ok(())
    }

    /// Update the theme mode setting
    pub fn set_theme_mode(&self, theme_mode: String) {
        let mut settings = self.settings.lock().unwrap();

        if settings.theme_mode != theme_mode {
            info!("Theme mode updated to: {}", theme_mode);
            settings.theme_mode = theme_mode.clone();
            *self.dirty.lock().unwrap() = true;
        }
    }

    /// Update accessibility settings
    pub fn set_accessibility(&self, high_contrast: bool, reduced_motion: bool, large_text: bool) {
        let mut settings = self.settings.lock().unwrap();

        let changed = settings.high_contrast != high_contrast
            || settings.reduced_motion != reduced_motion
            || settings.large_text != large_text;

        if changed {
            settings.high_contrast = high_contrast;
            settings.reduced_motion = reduced_motion;
            settings.large_text = large_text;
            *self.dirty.lock().unwrap() = true;
            info!("Accessibility settings updated");
        }
    }

    /// Update terminal settings including font
    pub fn set_terminal_settings(
        &self,
        font_family: String,
        font_size: f32,
        font_zoom: f32,
        use_webgl: bool,
        auto_scroll: bool,
        copy_on_select: bool,
    ) {
        let mut settings = self.settings.lock().unwrap();

        let changed = settings.terminal_font_family != font_family
            || settings.terminal_font_size != font_size
            || settings.terminal_font_zoom != font_zoom
            || settings.terminal_use_webgl != use_webgl
            || settings.terminal_auto_scroll != auto_scroll
            || settings.terminal_copy_on_select != copy_on_select;

        if changed {
            settings.terminal_font_family = font_family;
            settings.terminal_font_size = font_size;
            settings.terminal_font_zoom = font_zoom;
            settings.terminal_use_webgl = use_webgl;
            settings.terminal_auto_scroll = auto_scroll;
            settings.terminal_copy_on_select = copy_on_select;
            *self.dirty.lock().unwrap() = true;
            info!("Terminal settings updated");
        }
    }

    /// Get terminal font settings
    pub fn get_terminal_font_settings(&self) -> (String, f32, f32) {
        let settings = self.settings.lock().unwrap();
        (
            settings.terminal_font_family.clone(),
            settings.terminal_font_size,
            settings.terminal_font_zoom,
        )
    }

    /// Check if settings need to be saved
    pub fn is_dirty(&self) -> bool {
        *self.dirty.lock().unwrap()
    }

    /// Save settings if dirty
    pub fn save_if_dirty(&self) -> anyhow::Result<()> {
        if self.is_dirty() {
            let settings = self.settings.lock().unwrap().clone();
            settings.save()?;
            *self.dirty.lock().unwrap() = false;
        }
        Ok(())
    }

    /// Force save settings
    pub fn force_save(&self) -> anyhow::Result<()> {
        let settings = self.settings.lock().unwrap().clone();
        settings.save()?;
        *self.dirty.lock().unwrap() = false;
        Ok(())
    }

    /// Get the current language code
    pub fn get_language(&self) -> String {
        self.settings.lock().unwrap().language.clone()
    }

    /// Get the current theme mode
    pub fn get_theme_mode(&self) -> String {
        self.settings.lock().unwrap().theme_mode.clone()
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.language, "en");
        assert_eq!(settings.theme_mode, "dark");
        assert!(!settings.high_contrast);
        assert!(settings.terminal_use_webgl);
    }

    #[test]
    fn test_settings_path() {
        let path = AppSettings::settings_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("easyssh"));
        assert!(path.to_string_lossy().contains("settings.json"));
    }
}
