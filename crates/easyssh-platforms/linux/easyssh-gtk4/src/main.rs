use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

mod app;
mod application;
mod bridge;
mod dialogs;
mod enhanced_app;
mod models;
mod server_detail;
mod server_list;
mod settings;
mod sidebar;
mod terminal_launcher;
mod theme;
mod tray;
mod views;
mod widgets;
mod window;

use application::EasySSHApplication;

const APP_ID: &str = "com.easyssh.EasySSH";
const VERSION: &str = "0.3.0";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("easyssh_gtk4=debug")
        .init();

    tracing::info!("Starting EasySSH Lite GTK4 v{}", VERSION);

    // Check display server type (Wayland vs X11)
    check_display_server();

    // Initialize GTK and libadwaita
    gtk4::init().expect("Failed to initialize GTK");
    libadwaita::init();

    // Load CSS styles
    load_css();

    // Create and run application
    let app = EasySSHApplication::new();

    app.run()
}

fn check_display_server() {
    if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
        tracing::info!("Running on Wayland: {}", wayland_display);
    } else if let Ok(display) = std::env::var("DISPLAY") {
        tracing::info!("Running on X11: {}", display);
    } else {
        tracing::warn!("No display server detected");
    }

    // Check for forced backends
    if let Ok(backend) = std::env::var("GDK_BACKEND") {
        tracing::info!("Forced GDK backend: {}", backend);
    }
}

fn load_css() {
    let provider = gtk4::CssProvider::new();

    let css_data = include_str!("styles.css");
    provider.load_from_data(css_data);

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    tracing::info!("CSS styles loaded successfully");
}

/// Get the current GTK theme variant (dark/light)
pub fn get_theme_variant() -> &'static str {
    if let Some(settings) = gtk4::Settings::default() {
        if settings.is_gtk_application_prefer_dark_theme() {
            "dark"
        } else {
            "light"
        }
    } else {
        "light"
    }
}
