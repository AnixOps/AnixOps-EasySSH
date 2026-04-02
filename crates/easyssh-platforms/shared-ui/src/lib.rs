/**
 * EasySSH Shared UI - Cross-Platform Component Library
 *
 * This crate provides:
 * - Theme system with light/dark/high-contrast modes
 * - Animation utilities with reduced-motion support
 * - Icon system with platform-native fallbacks
 * - Layout primitives for responsive design
 * - Accessibility helpers (WCAG 2.1 AA compliant)
 *
 * Platform support:
 * - Windows: egui native rendering
 * - Linux: GTK4 native rendering
 * - macOS: SwiftUI native rendering
 */

pub mod accessibility;
pub mod animations;
pub mod components;
pub mod icons;
pub mod layout;
pub mod theme;

pub use theme::{Theme, ThemeManager, ColorScheme};
pub use animations::{Animation, AnimationManager, Easing};
pub use accessibility::{AccessibilityManager, A11yProps};
pub use icons::{Icon, IconSet, IconSize};
pub use layout::{ResponsiveLayout, Breakpoint, Spacing};

use std::sync::Arc;

/// Shared UI library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Global UI configuration
#[derive(Debug, Clone)]
pub struct UIConfig {
    /// Theme configuration
    pub theme: theme::ThemeConfig,
    /// Animation preferences
    pub animations: animations::AnimationConfig,
    /// Accessibility settings
    pub accessibility: accessibility::AccessibilityConfig,
    /// Layout breakpoints
    pub layout: layout::LayoutConfig,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: theme::ThemeConfig::default(),
            animations: animations::AnimationConfig::default(),
            accessibility: accessibility::AccessibilityConfig::default(),
            layout: layout::LayoutConfig::default(),
        }
    }
}

/// Main UI manager that coordinates all subsystems
pub struct UIManager {
    config: UIConfig,
    theme_manager: Arc<theme::ThemeManager>,
    animation_manager: Arc<animations::AnimationManager>,
    accessibility_manager: Arc<accessibility::AccessibilityManager>,
}

impl UIManager {
    /// Create a new UI manager with default configuration
    pub fn new() -> Self {
        let config = UIConfig::default();
        Self::with_config(config)
    }

    /// Create a new UI manager with custom configuration
    pub fn with_config(config: UIConfig) -> Self {
        let theme_manager = Arc::new(theme::ThemeManager::new(&config.theme));
        let animation_manager = Arc::new(animations::AnimationManager::new(&config.animations));
        let accessibility_manager = Arc::new(accessibility::AccessibilityManager::new(&config.accessibility));

        Self {
            config,
            theme_manager,
            animation_manager,
            accessibility_manager,
        }
    }

    /// Get the theme manager
    pub fn theme(&self) -> &theme::ThemeManager {
        &self.theme_manager
    }

    /// Get the animation manager
    pub fn animations(&self) -> &animations::AnimationManager {
        &self.animation_manager
    }

    /// Get the accessibility manager
    pub fn accessibility(&self) -> &accessibility::AccessibilityManager {
        &self.accessibility_manager
    }

    /// Update configuration at runtime
    pub fn update_config(&mut self, config: UIConfig) {
        self.config = config.clone();
        // Note: Subsystem updates would require internal mutability
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_manager_creation() {
        let ui = UIManager::new();
        assert!(!ui.config.animations.reduced_motion);
    }

    #[test]
    fn test_default_config() {
        let config = UIConfig::default();
        assert_eq!(config.theme.color_scheme, theme::ColorScheme::System);
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
