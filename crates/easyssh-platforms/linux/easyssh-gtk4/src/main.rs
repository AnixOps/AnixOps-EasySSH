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
mod sidebar;
mod terminal_launcher;
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

    // Initialize GTK and libadwaita
    gtk4::init().expect("Failed to initialize GTK");
    libadwaita::init();

    // Load CSS styles
    load_css();

    // Create and run application
    let app = EasySSHApplication::new();

    app.run()
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
}
