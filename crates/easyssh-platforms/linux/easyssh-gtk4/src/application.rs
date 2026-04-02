use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::window::EasySSHWindow;
use crate::models::AppState;

const APP_ID: &str = "com.easyssh.EasySSH";

pub struct EasySSHApplication {
    app: adw::Application,
    state: Arc<RefCell<AppState>>,
    window: RefCell<Option<Arc<EasySSHWindow>>>,
}

impl EasySSHApplication {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id(APP_ID)
            .build();

        let state = Arc::new(RefCell::new(AppState::new()));
        let window_ref: RefCell<Option<Arc<EasySSHWindow>>> = RefCell::new(None);

        let app_weak = app.downgrade();
        app.connect_startup(move |_| {
            tracing::info!("EasySSH application startup");
            setup_actions(&app_weak);
        });

        let state_clone = state.clone();
        let window_ref_clone = window_ref.clone();
        app.connect_activate(move |app| {
            tracing::info!("EasySSH application activated");

            if let Some(window) = window_ref_clone.borrow().as_ref() {
                window.present();
                return;
            }

            let window = Arc::new(EasySSHWindow::new(app, state_clone.clone()));
            window.present();
            window_ref_clone.replace(Some(window));
        });

        app.connect_shutdown(move |_| {
            tracing::info!("EasySSH application shutting down");
        });

        Self { app, state, window: window_ref }
    }

    pub fn run(&self) -> glib::ExitCode {
        tracing::info!("Running EasySSH application");
        self.app.run()
    }
}

fn setup_actions(app: &glib::WeakRef<adw::Application>) {
    // About action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        show_about_dialog();
    });

    // Preferences action
    let prefs_action = gio::SimpleAction::new("preferences", None);
    prefs_action.connect_activate(move |_, _| {
        tracing::info!("Preferences activated");
        // TODO: Implement preferences dialog
    });

    // Quit action
    let quit_action = gio::SimpleAction::new("quit", None);
    let app_clone = app.clone();
    quit_action.connect_activate(move |_, _| {
        if let Some(app) = app_clone.upgrade() {
            app.quit();
        }
    });
    quit_action.set_enabled(true);

    // Add keyboard shortcut for quit
    if let Some(app) = app.upgrade() {
        app.set_accels_for_action("app.quit", &["<primary>q"]);
        app.set_accels_for_action("app.preferences", &["<primary>comma"]);
        app.add_action(&about_action);
        app.add_action(&prefs_action);
        app.add_action(&quit_action);
    }
}

fn show_about_dialog() {
    let dialog = adw::AboutWindow::builder()
        .application_name("EasySSH")
        .application_icon("com.easyssh.EasySSH")
        .developer_name("EasySSH Team")
        .version("0.3.0")
        .website("https://github.com/easyssh/easyssh")
        .issue_url("https://github.com/easyssh/easyssh/issues")
        .support_url("https://github.com/easyssh/easyssh/discussions")
        .copyright("© 2026 EasySSH Team")
        .license_type(gtk4::License::MitX11)
        .comments("A lightweight SSH client for Linux")
        .build();

    dialog.add_acknowledgement_section(Some("Contributors"), &[
        "EasySSH Team",
    ]);

    dialog.present();
}
