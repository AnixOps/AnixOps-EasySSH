use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::models::AppState;
use crate::theme::{init_theme_support, ThemeManager};
use crate::tray::{init_notifications, SystemTray};
use crate::window::EasySSHWindow;

const APP_ID: &str = "com.easyssh.EasySSH";
const VERSION: &str = "0.3.0";

pub struct EasySSHApplication {
    app: adw::Application,
    state: Arc<RefCell<AppState>>,
    window: RefCell<Option<Arc<EasySSHWindow>>>,
    theme_manager: ThemeManager,
    tray: RefCell<Option<SystemTray>>,
}

impl EasySSHApplication {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id(APP_ID)
            .flags(gio::ApplicationFlags::HANDLES_OPEN)
            .build();

        let state = Arc::new(RefCell::new(AppState::new()));
        let window_ref: RefCell<Option<Arc<EasySSHWindow>>> = RefCell::new(None);
        let theme_manager = ThemeManager::new();
        let tray: RefCell<Option<SystemTray>> = RefCell::new(None);

        let app_weak = app.downgrade();
        app.connect_startup(move |app| {
            tracing::info!("EasySSH application startup");

            // Initialize theme support
            init_theme_support(app);

            // Initialize notifications
            init_notifications(app);

            // Set up actions
            setup_actions(&app_weak);
        });

        let state_clone = state.clone();
        let window_ref_clone = window_ref.clone();
        let tray_clone = tray.clone();

        app.connect_activate(move |app| {
            tracing::info!("EasySSH application activated");

            if let Some(window) = window_ref_clone.borrow().as_ref() {
                window.present();
                return;
            }

            let window = Arc::new(EasySSHWindow::new(app, state_clone.clone()));
            window.present();
            window_ref_clone.replace(Some(window));

            // Initialize tray after window is created
            if let Some(tray_instance) = SystemTray::new() {
                tray_instance.connect_activate(glib::clone!(@weak window => move || {
                    window.present();
                }));
                tray_clone.replace(Some(tray_instance));
            }
        });

        app.connect_shutdown(move |_| {
            tracing::info!("EasySSH application shutting down");
        });

        Self {
            app,
            state,
            window: window_ref,
            theme_manager,
            tray,
        }
    }

    pub fn run(&self) -> glib::ExitCode {
        tracing::info!("Running EasySSH application");
        self.app.run()
    }

    pub fn app(&self) -> &adw::Application {
        &self.app
    }

    pub fn show_notification(&self, title: &str, body: &str) {
        if let Some(ref tray) = *self.tray.borrow() {
            tray.show_notification(title, body);
        }
    }
}

fn setup_actions(app: &glib::WeakRef<adw::Application>) {
    // About action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        show_about_dialog();
    });

    // Help action
    let help_action = gio::SimpleAction::new("help", None);
    help_action.connect_activate(move |_, _| {
        tracing::info!("Help activated");
        let _ = open::that("https://github.com/easyssh/easyssh/blob/main/docs/README.md");
    });

    // Preferences action
    let prefs_action = gio::SimpleAction::new("preferences", None);
    prefs_action.connect_activate(move |_, _| {
        tracing::info!("Preferences activated");
        show_preferences_dialog();
    });

    // Color scheme toggle
    let color_scheme_action =
        gio::SimpleAction::new_stateful("toggle-color-scheme", None, &"default".to_variant());
    color_scheme_action.connect_activate(|action, _| {
        let style_manager = adw::StyleManager::default();
        let current = style_manager.color_scheme();

        let next = match current {
            adw::ColorScheme::Default | adw::ColorScheme::PreferLight => {
                adw::ColorScheme::PreferDark
            }
            adw::ColorScheme::PreferDark => adw::ColorScheme::PreferLight,
            _ => adw::ColorScheme::Default,
        };

        style_manager.set_color_scheme(next);
        tracing::info!("Color scheme toggled to: {:?}", next);

        let scheme_name = match next {
            adw::ColorScheme::PreferDark => "dark",
            _ => "light",
        };
        action.set_state(&scheme_name.to_variant());
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

    // Add keyboard shortcuts
    if let Some(app) = app.upgrade() {
        app.set_accels_for_action("app.quit", &["<primary>q"]);
        app.set_accels_for_action("app.preferences", &["<primary>comma"]);
        app.set_accels_for_action("app.help", &["F1"]);
        app.set_accels_for_action("app.toggle-color-scheme", &["<primary>t"]);

        app.add_action(&about_action);
        app.add_action(&help_action);
        app.add_action(&prefs_action);
        app.add_action(&color_scheme_action);
        app.add_action(&quit_action);
    }
}

fn show_about_dialog() {
    let dialog = adw::AboutWindow::builder()
        .application_name("EasySSH")
        .application_icon("com.easyssh.EasySSH")
        .developer_name("EasySSH Team")
        .version(VERSION)
        .website("https://github.com/easyssh/easyssh")
        .issue_url("https://github.com/easyssh/easyssh/issues")
        .support_url("https://github.com/easyssh/easyssh/discussions")
        .copyright("© 2026 EasySSH Team")
        .license_type(gtk4::License::MitX11)
        .comments("A lightweight SSH client for Linux")
        .build();

    // Add acknowledgments
    dialog.add_acknowledgement_section(Some("Contributors"), &["EasySSH Team"]);
    dialog.add_legal_section(
        "GTK4",
        Some("GTK4 and libadwaita are used for the user interface"),
        gtk4::License::Lgpl2_1,
        None,
    );

    dialog.present();
}

fn show_preferences_dialog() {
    // Create preferences dialog with adwaita patterns
    let dialog = adw::PreferencesWindow::builder()
        .title("Preferences")
        .default_width(600)
        .default_height(500)
        .modal(true)
        .destroy_with_parent(true)
        .build();

    // General preferences page
    let general_page = adw::PreferencesPage::builder()
        .title("General")
        .icon_name("emblem-system-symbolic")
        .build();

    // Appearance group
    let appearance_group = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Customize the look and feel")
        .build();

    let style_row = adw::ComboRow::builder()
        .title("Style")
        .subtitle("Preferred color scheme")
        .model(&gtk4::StringList::new(&["System Default", "Light", "Dark"]))
        .build();

    style_row.connect_selected_notify(|row| {
        let style_manager = adw::StyleManager::default();
        let scheme = match row.selected() {
            1 => adw::ColorScheme::PreferLight,
            2 => adw::ColorScheme::PreferDark,
            _ => adw::ColorScheme::Default,
        };
        style_manager.set_color_scheme(scheme);
    });

    appearance_group.add(&style_row);
    general_page.add(&appearance_group);

    // Terminal group
    let terminal_group = adw::PreferencesGroup::builder()
        .title("Terminal")
        .description("Terminal emulator settings")
        .build();

    let auto_detect_row = adw::SwitchRow::builder()
        .title("Auto-detect Terminal")
        .subtitle("Automatically detect the best available terminal emulator")
        .active(true)
        .build();

    terminal_group.add(&auto_detect_row);
    general_page.add(&terminal_group);

    dialog.add(&general_page);

    // Connections page
    let connections_page = adw::PreferencesPage::builder()
        .title("Connections")
        .icon_name("network-wired-symbolic")
        .build();

    let connection_group = adw::PreferencesGroup::builder()
        .title("SSH Settings")
        .description("Default SSH connection settings")
        .build();

    let timeout_row = adw::SpinRow::builder()
        .title("Connection Timeout")
        .subtitle("Timeout in seconds for connection attempts")
        .adjustment(&gtk4::Adjustment::new(30.0, 5.0, 300.0, 5.0, 10.0, 0.0))
        .build();

    connection_group.add(&timeout_row);
    connections_page.add(&connection_group);

    dialog.add(&connections_page);

    dialog.present();
}
