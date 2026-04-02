use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::dialogs;
use crate::models::{AppState, Server, ServerGroup};
use crate::server_detail::ServerDetail;
use crate::server_list::ServerList;
use crate::sidebar::Sidebar;
use crate::theme::ThemeManager;
use crate::tray::{NotificationManager, SystemTray};

pub struct EasySSHWindow {
    window: adw::ApplicationWindow,
    state: Arc<RefCell<AppState>>,
    sidebar: Sidebar,
    server_list: ServerList,
    server_detail: ServerDetail,
    paned: gtk4::Paned,
    theme_manager: ThemeManager,
    notification_manager: Option<NotificationManager>,
}

impl EasySSHWindow {
    pub fn new(app: &adw::Application, state: Arc<RefCell<AppState>>) -> Self {
        // Initialize theme manager
        let theme_manager = ThemeManager::new();

        // Create header bar with GNOME HIG compliant design
        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(true);

        // Add button with keyboard shortcut
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some("Add Server (Ctrl+N)"));
        add_button.set_accessible_role(gtk4::AccessibleRole::Button);
        add_button.set_accessible_label(Some("Add new server"));
        header.pack_start(&add_button);

        // Add group button
        let add_group_button = gtk4::Button::from_icon_name("folder-new-symbolic");
        add_group_button.set_tooltip_text(Some("Add Group (Ctrl+G)"));
        add_group_button.set_accessible_label(Some("Add new server group"));
        header.pack_start(&add_group_button);

        // Search entry - centered in header
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search servers..."));
        search_entry.set_width_request(320);
        search_entry.set_accessible_role(gtk4::AccessibleRole::SearchBox);
        search_entry.set_accessible_label(Some("Search servers by name, host, or username"));
        header.set_title_widget(Some(&search_entry));

        // Primary menu - GNOME HIG compliant
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Main Menu"));
        menu_button.set_accessible_label(Some("Main menu"));

        // Build menu with proper structure
        let menu = gio::Menu::new();

        // View section
        let view_section = gio::Menu::new();
        view_section.append(Some("Refresh"), Some("win.refresh"));
        view_section.append(Some("Toggle Sidebar"), Some("win.toggle-sidebar"));
        menu.append_section(Some("View"), &view_section);

        // Settings section
        let settings_section = gio::Menu::new();
        settings_section.append(Some("Preferences"), Some("app.preferences"));
        settings_section.append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
        menu.append_section(Some("Settings"), &settings_section);

        // Help section
        let help_section = gio::Menu::new();
        help_section.append(Some("Help"), Some("app.help"));
        help_section.append(Some("About EasySSH"), Some("app.about"));
        menu.append_section(Some("Help"), &help_section);

        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        // Create sidebar (groups/tags) with navigation pattern
        let sidebar = Sidebar::new();
        let sidebar_scroll = gtk4::ScrolledWindow::builder()
            .child(&sidebar.widget())
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .width_request(220)
            .propagate_natural_width(true)
            .build();

        // Create server list with modern styling
        let server_list = ServerList::new();
        let list_scroll = gtk4::ScrolledWindow::builder()
            .child(&server_list.widget())
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .width_request(320)
            .propagate_natural_width(true)
            .build();

        // Create server detail view
        let server_detail = ServerDetail::new();

        // Create inner paned (server list + detail)
        let inner_paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        inner_paned.set_start_child(Some(&list_scroll));
        inner_paned.set_end_child(Some(server_detail.widget()));
        inner_paned.set_resize_start_child(true);
        inner_paned.set_shrink_start_child(false);
        inner_paned.set_resize_end_child(true);
        inner_paned.set_shrink_end_child(false);
        inner_paned.set_position(320);
        inner_paned.set_wide_handle(true);

        // Create outer paned (sidebar + inner content)
        let outer_paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        outer_paned.set_start_child(Some(&sidebar_scroll));
        outer_paned.set_end_child(Some(&inner_paned));
        outer_paned.set_resize_start_child(true);
        outer_paned.set_shrink_start_child(false);
        outer_paned.set_resize_end_child(true);
        outer_paned.set_shrink_end_child(false);
        outer_paned.set_position(220);
        outer_paned.set_wide_handle(true);

        // Create toolbar view with header
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&outer_paned));

        // Create window with proper sizing for GNOME HIG
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("EasySSH")
            .default_width(1280)
            .default_height(800)
            .content(&toolbar_view)
            .build();

        // Set minimum window size
        window.set_size_request(800, 600);

        // Add responsive breakpoints for GNOME HIG
        let narrow_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            700.0,
            adw::LengthUnit::Px,
        ));
        narrow_breakpoint.add_setter(&outer_paned, "collapsed", &true.to_value());
        narrow_breakpoint.add_setter(&sidebar_scroll, "visible", &false.to_value());
        window.add_breakpoint(&narrow_breakpoint);

        let medium_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            900.0,
            adw::LengthUnit::Px,
        ));
        medium_breakpoint.add_setter(&outer_paned, "position", &180i32.to_value());
        window.add_breakpoint(&medium_breakpoint);

        // Initialize notification manager
        let notification_manager = Some(NotificationManager::new(&app.clone()));

        let window_obj = Self {
            window,
            state: state.clone(),
            sidebar,
            server_list,
            server_detail,
            paned: outer_paned,
            theme_manager,
            notification_manager,
        };

        window_obj.setup_actions(app);
        window_obj.setup_signals(&add_button, &add_group_button, &search_entry);
        window_obj.setup_keyboard_shortcuts();
        window_obj.setup_theme_monitoring();
        window_obj.load_data();

        window_obj
    }

    fn setup_actions(&self, app: &adw::Application) {
        let window_weak = self.window.downgrade();

        // Refresh action
        let refresh_action = gio::SimpleAction::new("refresh", None);
        refresh_action.connect_activate(glib::clone!(@weak self.state as state => move |_, _| {
            tracing::info!("Refreshing server list");
            // TODO: Implement actual refresh logic
        }));
        self.window.add_action(&refresh_action);
        app.set_accels_for_action("win.refresh", &["<primary>r", "F5"]);

        // Toggle sidebar action
        let toggle_sidebar_action = gio::SimpleAction::new("toggle-sidebar", None);
        let sidebar_weak = self.sidebar.widget().downgrade();
        toggle_sidebar_action.connect_activate(move |_, _| {
            if let Some(sidebar) = sidebar_weak.upgrade() {
                let visible = sidebar.is_visible();
                sidebar.set_visible(!visible);
            }
        });
        self.window.add_action(&toggle_sidebar_action);
        app.set_accels_for_action("win.toggle-sidebar", &["F9"]);

        // Help action
        let help_action = gio::SimpleAction::new("help", None);
        help_action.connect_activate(move |_, _| {
            tracing::info!("Opening help");
            // Open help URL
            let _ = open::that("https://github.com/easyssh/easyssh/blob/main/docs/README.md");
        });
        app.add_action(&help_action);

        // Set up keyboard shortcuts window
        let shortcuts_window = self.build_shortcuts_window();
        self.window.set_help_overlay(Some(&shortcuts_window));
    }

    fn setup_signals(
        &self,
        add_button: &gtk4::Button,
        add_group_button: &gtk4::Button,
        search_entry: &gtk4::SearchEntry,
    ) {
        let state = self.state.clone();

        // Add server button
        add_button.connect_clicked(glib::clone!(@weak self.window as window, @strong state => move |_| {
            dialogs::show_add_server_dialog(&window, glib::clone!(@strong state => move |server| {
                state.borrow_mut().add_server(server);
            }));
        }));

        // Add group button
        add_group_button.connect_clicked(
            glib::clone!(@weak self.window as window, @strong state => move |_| {
                dialogs::show_add_group_dialog(&window, glib::clone!(@strong state => move |group| {
                    state.borrow_mut().add_group(group);
                }));
            }),
        );

        // Sidebar group selection
        self.sidebar.connect_group_selected(
            glib::clone!(@weak self.server_list as list, @strong state => move |group_id| {
                let servers = if let Some(ref id) = group_id {
                    state.borrow().get_servers_by_group(id)
                } else {
                    state.borrow().get_servers()
                };
                list.set_servers(servers);
                state.borrow_mut().set_selected_group(group_id);
            }),
        );

        // Server list selection
        self.server_list.connect_server_selected(
            glib::clone!(@weak self.server_detail as detail, @strong state => move |server| {
                detail.set_server(Some(&server));
                state.borrow_mut().set_selected_server(Some(server.id.clone()));
            }),
        );

        // Server connect
        self.server_list.connect_server_connect(
            glib::clone!(@weak state, @weak self.notification_manager as notif => move |server| {
                crate::terminal_launcher::launch_terminal(&server);

                // Show notification
                if let Some(ref nm) = notif {
                    nm.notify_connection(&server.name, true);
                }
            }),
        );

        // Server edit
        self.server_list.connect_server_edit(
            glib::clone!(@weak self.window as window, @strong state => move |server| {
                dialogs::show_edit_server_dialog(&window, &server, glib::clone!(@strong state => move |updated| {
                    state.borrow_mut().update_server(updated);
                }));
            }),
        );

        // Server delete
        self.server_list.connect_server_delete(
            glib::clone!(@weak self.window as window, @strong state => move |server| {
                dialogs::show_confirm_delete_dialog(&window, &server.name, glib::clone!(@strong state => move || {
                    state.borrow_mut().delete_server(&server.id);
                }));
            }),
        );

        // Search with debouncing
        let search_timeout = RefCell::new(None::<glib::SourceId>);
        search_entry.connect_search_changed(
            glib::clone!(@weak self.server_list as list, @strong state => move |entry| {
                // Cancel previous timeout
                if let Some(timeout) = search_timeout.borrow_mut().take() {
                    timeout.remove();
                }

                let query = entry.text().to_string().to_lowercase();

                // Debounce search for better performance
                let timeout = glib::timeout_add_local_once(
                    std::time::Duration::from_millis(150),
                    glib::clone!(@weak list, @strong state => move || {
                        let servers: Vec<crate::models::Server> = state
                            .borrow()
                            .get_servers()
                            .into_iter()
                            .filter(|s| {
                                s.name.to_lowercase().contains(&query)
                                    || s.host.to_lowercase().contains(&query)
                                    || s.username.to_lowercase().contains(&query)
                            })
                            .collect();
                        list.set_servers(servers);
                    }),
                );

                *search_timeout.borrow_mut() = Some(timeout);
            }),
        );

        // Detail view signals
        self.server_detail.connect_connect_clicked(
            glib::clone!(@strong state, @weak self.notification_manager as notif => move |server| {
                crate::terminal_launcher::launch_terminal(&server);

                if let Some(ref nm) = notif {
                    nm.notify_connection(&server.name, true);
                }
            }),
        );

        self.server_detail.connect_edit_clicked(
            glib::clone!(@weak self.window as window, @strong state => move |server| {
                dialogs::show_edit_server_dialog(&window, &server, glib::clone!(@strong state => move |updated| {
                    state.borrow_mut().update_server(updated);
                }));
            }),
        );

        self.server_detail.connect_delete_clicked(
            glib::clone!(@weak self.window as window, @strong state => move |server| {
                dialogs::show_confirm_delete_dialog(&window, &server.name, glib::clone!(@strong state => move || {
                    state.borrow_mut().delete_server(&server.id);
                }));
            }),
        );
    }

    fn setup_keyboard_shortcuts(&self) {
        let controller = gtk4::EventControllerKey::new();
        let state = self.state.clone();
        let server_list = self.server_list.clone();
        let server_detail = self.server_detail.clone();
        let window_weak = self.window.downgrade();

        controller.connect_key_pressed(
            glib::clone!(@strong state, @strong server_list, @strong server_detail, @strong window_weak =>
                move |_, key, _, modifier| {
                // Global shortcuts
                if modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    match key {
                        gtk4::gdk::Key::n | gtk4::gdk::Key::N => {
                            // Add server
                            if let Some(window) = window_weak.upgrade() {
                                dialogs::show_add_server_dialog(
                                    &window,
                                    glib::clone!(@strong state => move |server| {
                                        state.borrow_mut().add_server(server);
                                    }),
                                );
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::g | gtk4::gdk::Key::G => {
                            // Add group
                            if let Some(window) = window_weak.upgrade() {
                                dialogs::show_add_group_dialog(
                                    &window,
                                    glib::clone!(@strong state => move |group| {
                                        state.borrow_mut().add_group(group);
                                    }),
                                );
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::d | gtk4::gdk::Key::D => {
                            // Connect to selected server
                            if let Some(server) = state.borrow().get_selected_server() {
                                crate::terminal_launcher::launch_terminal(&server);
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::e | gtk4::gdk::Key::E => {
                            // Edit selected server
                            if let Some(server) = state.borrow().get_selected_server() {
                                if let Some(window) = window_weak.upgrade() {
                                    dialogs::show_edit_server_dialog(
                                        &window,
                                        &server,
                                        glib::clone!(@strong state => move |updated| {
                                            state.borrow_mut().update_server(updated);
                                        }),
                                    );
                                }
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                            // Focus search
                            // TODO: Focus search entry
                            glib::Propagation::Stop
                        }
                        _ => glib::Propagation::Proceed,
                    }
                } else {
                    // Single key shortcuts (no modifier)
                    match key {
                        gtk4::gdk::Key::F2 => {
                            // Edit on F2
                            if let Some(server) = state.borrow().get_selected_server() {
                                if let Some(window) = window_weak.upgrade() {
                                    dialogs::show_edit_server_dialog(
                                        &window,
                                        &server,
                                        glib::clone!(@strong state => move |updated| {
                                            state.borrow_mut().update_server(updated);
                                        }),
                                    );
                                }
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::Delete | gtk4::gdk::Key::KP_Delete => {
                            // Delete key without modifier
                            if let Some(server) = state.borrow().get_selected_server() {
                                if let Some(window) = window_weak.upgrade() {
                                    dialogs::show_confirm_delete_dialog(
                                        &window,
                                        &server.name,
                                        glib::clone!(@strong state => move || {
                                            state.borrow_mut().delete_server(&server.id);
                                            server_detail.set_server(None);
                                        }),
                                    );
                                }
                            }
                            glib::Propagation::Stop
                        }
                        _ => glib::Propagation::Proceed,
                    }
                }
            }),
        );

        self.window.add_controller(controller);
    }

    fn setup_theme_monitoring(&self) {
        // Connect to theme changes
        self.theme_manager.connect_dark_mode_changed(|is_dark| {
            tracing::info!(
                "Theme changed to {}",
                if is_dark { "dark" } else { "light" }
            );
        });
    }

    fn load_data(&self) {
        // Load groups into sidebar
        let groups = self.state.borrow().get_groups();
        self.sidebar.set_groups(groups);

        // Load servers
        let servers = self.state.borrow().get_servers();
        self.server_list.set_servers(servers);
    }

    fn build_shortcuts_window(&self) -> gtk4::ShortcutsWindow {
        let shortcuts = gtk4::ShortcutsWindow::builder()
            .modal(true)
            .destroy_with_parent(true)
            .build();

        // Create sections for different shortcut categories
        let general_section = gtk4::ShortcutsSection::builder()
            .title("General")
            .show(true)
            .build();

        let group = gtk4::ShortcutsGroup::builder().title("Application").build();

        // Add shortcuts
        let shortcuts_data = vec![
            ("Add Server", "<primary>n", "Add a new server"),
            ("Add Group", "<primary>g", "Add a new group"),
            ("Connect", "<primary>d", "Connect to selected server"),
            ("Edit", "F2", "Edit selected server"),
            ("Delete", "Delete", "Delete selected server"),
            ("Search", "<primary>f", "Focus search box"),
            ("Refresh", "<primary>r", "Refresh server list"),
            ("Preferences", "<primary>comma", "Open preferences"),
            (
                "Keyboard Shortcuts",
                "<primary>question",
                "Show this dialog",
            ),
            ("Quit", "<primary>q", "Quit application"),
        ];

        for (title, accelerator, subtitle) in shortcuts_data {
            let shortcut = gtk4::ShortcutsShortcut::builder()
                .title(title)
                .accelerator(accelerator)
                .subtitle(subtitle)
                .build();
            group.append(&shortcut);
        }

        general_section.append(&group);
        shortcuts.add_section(&general_section);

        shortcuts
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn widget(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    pub fn show_notification(&self, title: &str, body: &str) {
        if let Some(ref nm) = self.notification_manager {
            nm.notify_info(title, body);
        }
    }
}

impl Clone for EasySSHWindow {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            state: self.state.clone(),
            sidebar: self.sidebar.clone(),
            server_list: self.server_list.clone(),
            server_detail: self.server_detail.clone(),
            paned: self.paned.clone(),
            theme_manager: ThemeManager::new(),
            notification_manager: None,
        }
    }
}
