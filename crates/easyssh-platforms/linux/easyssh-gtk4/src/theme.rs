use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

/// Theme manager for handling dark/light mode switching
pub struct ThemeManager {
    settings: gtk4::Settings,
    style_manager: adw::StyleManager,
    dark_mode_callback: RefCell<Option<Box<dyn Fn(bool) + 'static>>>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let settings = gtk4::Settings::default().expect("Failed to get GTK settings");
        let style_manager = adw::StyleManager::default();

        let manager = Self {
            settings,
            style_manager,
            dark_mode_callback: RefCell::new(None),
        };

        manager.setup_monitoring();
        manager
    }

    fn setup_monitoring(&self) {
        let callback_cell = self.dark_mode_callback.clone();

        self.settings
            .connect_gtk_application_prefer_dark_theme_notify(move |settings| {
                let is_dark = settings.is_gtk_application_prefer_dark_theme();
                tracing::info!("Theme changed: {}", if is_dark { "dark" } else { "light" });

                if let Some(ref callback) = *callback_cell.borrow() {
                    callback(is_dark);
                }
            });
    }

    /// Check if dark mode is active
    pub fn is_dark_mode(&self) -> bool {
        self.style_manager.is_dark()
    }

    /// Get current accent color
    pub fn accent_color(&self) -> gdk::RGBA {
        self.style_manager.accent_color()
    }

    /// Set color scheme manually
    pub fn set_color_scheme(&self, scheme: adw::ColorScheme) {
        self.style_manager.set_color_scheme(scheme);
        tracing::info!("Color scheme set to: {:?}", scheme);
    }

    /// Get current color scheme
    pub fn color_scheme(&self) -> adw::ColorScheme {
        self.style_manager.color_scheme()
    }

    /// Connect to theme changes
    pub fn connect_dark_mode_changed<F>(&self, callback: F)
    where
        F: Fn(bool) + 'static,
    {
        self.dark_mode_callback.replace(Some(Box::new(callback)));
    }

    /// Get system theme variant
    pub fn system_theme(&self) -> ThemeVariant {
        if self.style_manager.is_dark() {
            ThemeVariant::Dark
        } else {
            ThemeVariant::Light
        }
    }

    /// Force high contrast mode
    pub fn set_high_contrast(&self, enabled: bool) {
        self.settings.set_gtk_application_prefer_dark_theme(enabled);
        // Note: High contrast is typically handled by the system theme
    }

    /// Get CSS for current theme
    pub fn get_theme_css(&self) -> String {
        let is_dark = self.is_dark_mode();
        format!(
            r#"
            :root {{
                --theme-mode: {};
                --accent-rgb: {};
            }}
            "#,
            if is_dark { "dark" } else { "light" },
            if is_dark {
                "100, 200, 255"
            } else {
                "26, 95, 180"
            }
        )
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme variant enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeVariant {
    Light,
    Dark,
    HighContrast,
}

impl ThemeVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeVariant::Light => "light",
            ThemeVariant::Dark => "dark",
            ThemeVariant::HighContrast => "high-contrast",
        }
    }

    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeVariant::Dark)
    }
}

/// Get the current system color scheme preference
pub fn get_system_color_scheme() -> adw::ColorScheme {
    let style_manager = adw::StyleManager::default();
    style_manager.color_scheme()
}

/// Check if the system prefers dark mode
pub fn system_prefers_dark() -> bool {
    if let Some(settings) = gtk4::Settings::default() {
        settings.is_gtk_application_prefer_dark_theme()
    } else {
        false
    }
}

/// Apply theme-specific CSS classes to a widget
pub fn apply_theme_classes<W: IsA<gtk4::Widget>>(widget: &W, is_dark: bool) {
    if is_dark {
        widget.add_css_class("dark-theme");
        widget.remove_css_class("light-theme");
    } else {
        widget.add_css_class("light-theme");
        widget.remove_css_class("dark-theme");
    }
}

/// Get accent color as hex string
pub fn get_accent_color_hex() -> String {
    let style_manager = adw::StyleManager::default();
    let rgba = style_manager.accent_color();
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgba.red() * 255.0) as u8,
        (rgba.green() * 255.0) as u8,
        (rgba.blue() * 255.0) as u8
    )
}

/// Initialize theme support for the application
pub fn init_theme_support(app: &adw::Application) {
    let style_manager = adw::StyleManager::default();

    // Log initial theme
    let initial_theme = if style_manager.is_dark() {
        "dark"
    } else {
        "light"
    };
    tracing::info!("Initial system theme: {}", initial_theme);

    // Monitor system theme changes
    style_manager.connect_dark_notify(|manager| {
        let is_dark = manager.is_dark();
        tracing::info!(
            "System theme changed to: {}",
            if is_dark { "dark" } else { "light" }
        );
    });
}

/// Create a themed icon name based on current theme
pub fn themed_icon_name(base_name: &str, is_dark: bool) -> String {
    if is_dark {
        format!("{}-dark", base_name)
    } else {
        base_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_variant_as_str() {
        assert_eq!(ThemeVariant::Light.as_str(), "light");
        assert_eq!(ThemeVariant::Dark.as_str(), "dark");
        assert_eq!(ThemeVariant::HighContrast.as_str(), "high-contrast");
    }

    #[test]
    fn test_theme_variant_is_dark() {
        assert!(!ThemeVariant::Light.is_dark());
        assert!(ThemeVariant::Dark.is_dark());
        assert!(!ThemeVariant::HighContrast.is_dark());
    }
}
