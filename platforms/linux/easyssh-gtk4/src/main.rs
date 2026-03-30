use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::env;
use tracing::{info, warn};

mod app;
mod models;
mod views;
mod widgets;

use app::EasySSHApp;

const APP_ID: &str = "com.easyssh.EasySSH";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting EasySSH GTK4");

    // Initialize GTK/libadwaita
    gtk4::init().expect("Failed to initialize GTK");
    libadwaita::init();

    // Load CSS styles
    load_css();

    // Create and run app
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run()
}

fn load_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("styles.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &adw::Application) {
    let easy_app = EasySSHApp::new(app);
    easy_app.present();
}
