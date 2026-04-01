use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

mod app;
mod bridge;
mod enhanced_app;
mod models;
mod views;
mod widgets;

use app::EasySSHApp;

const APP_ID: &str = "com.easyssh.EasySSH";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting EasySSH GTK4");

    // Initialize GTK/libadwaita
    gtk4::init().expect("Failed to initialize GTK");
    libadwaita::init();

    // Load CSS styles
    load_css();

    // Create and run app
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    // Store EasySSHApp instance for shutdown handling
    let easy_app_ref: RefCell<Option<Arc<EasySSHApp>>> = RefCell::new(None);

    app.connect_activate(glib::clone!(@strong easy_app_ref => move |app| {
        let easy_app = Arc::new(EasySSHApp::new(app));
        easy_app.present();
        easy_app_ref.replace(Some(easy_app));
    }));

    // Handle graceful shutdown when the application is about to quit
    app.connect_shutdown(glib::clone!(@strong easy_app_ref => move |_| {
        tracing::info!("Application shutting down...");
        if let Some(easy_app) = easy_app_ref.borrow().as_ref() {
            easy_app.shutdown();
        }
        tracing::info!("Application shutdown complete");
    }));

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
