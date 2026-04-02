use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;

use crate::dialogs;
use crate::models::{AppState, Server, ServerGroup};
use crate::server_detail::ServerDetail;
use crate::server_list::ServerList;
use crate::sidebar::Sidebar;

pub struct EasySSHWindow {
    window: adw::ApplicationWindow,
    state: Arc<RefCell<AppState>>,
    sidebar: Sidebar,
    server_list: ServerList,
    server_detail: ServerDetail,
    paned: gtk4::Paned,
}

impl EasySSHWindow {
    pub fn new(app: &adw::Application, state: Arc<RefCell<AppState>>) -> Self {
        // Create header bar
        let header = adw::HeaderBar::new();

        // Add button
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some("Add Server (Ctrl+N)"));
        header.pack_start(&add_button);

        // Add group button
        let add_group_button = gtk4::Button::from_icon_name("folder-new-symbolic");
        add_group_button.set_tooltip_text(Some("Add Group"));
        header.pack_start(&add_group_button);

        // Search entry
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search servers..."));
        search_entry.set_width_request(300);
        header.set_title_widget(Some(&search_entry));

        // Menu button
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Menu"));

        // Create menu
        let menu = gio::Menu::new();
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
        menu.append(Some("About EasySSH"), Some("app.about"));
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        // Create sidebar (groups/tags)
        let sidebar = Sidebar::new();
        let sidebar_scroll = gtk4::ScrolledWindow::builder()
            .child(&sidebar.widget())
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .width_request(200)
            .build();

        // Create server list
        let server_list = ServerList::new();
        let list_scroll = gtk4::ScrolledWindow::builder()
            .child(&server_list.widget())
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .width_request(280)
            .build();

        // Create server detail
        let server_detail = ServerDetail::new();

        // Create inner paned (server list + detail)
        let inner_paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        inner_paned.set_start_child(Some(&list_scroll));
        inner_paned.set_end_child(Some(server_detail.widget()));
        inner_paned.set_resize_start_child(false);
        inner_paned.set_shrink_start_child(false);
        inner_paned.set_position(280);

        // Create outer paned (sidebar + inner content)
        let outer_paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        outer_paned.set_start_child(Some(&sidebar_scroll));
        outer_paned.set_end_child(Some(&inner_paned));
        outer_paned.set_resize_start_child(false);
        outer_paned.set_shrink_start_child(false);
        outer_paned.set_position(200);

        // Create toolbar view
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&outer_paned));

        // Create window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("EasySSH")
            .default_width(1200)
            .default_height(800)
            .content(&toolbar_view)
            .build();

        // Add breakpoint for responsive design
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            800.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&outer_paned, "collapsed", &true.to_value());
        window.add_breakpoint(&breakpoint);

        let window_obj = Self {
            window,
            state: state.clone(),
            sidebar,
            server_list,
            server_detail,
            paned: outer_paned,
        };

        window_obj.setup_signals(&add_button, &add_group_button, &search_entry);
        window_obj.setup_keyboard_shortcuts();
        window_obj.load_data();

        window_obj
    }

    fn setup_signals(
        &self,
        add_button: &gtk4::Button,
        add_group_button: &gtk4::Button,
        search_entry: &gtk4::SearchEntry,
    ) {
        let state = self.state.clone();

        // Add server button
        add_button.connect_clicked(glib::clone!(@weak self.window as window => move |_| {
            dialogs::show_add_server_dialog(&window, glib::clone!(@strong state => move |server| {
                state.borrow_mut().add_server(server);
            }));
        }));

        // Add group button
        add_group_button.connect_clicked(glib::clone!(@weak self.window as window => move |_| {
            dialogs::show_add_group_dialog(&window, glib::clone!(@strong state => move |group| {
                state.borrow_mut().add_group(group);
            }));
        }));

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
        self.server_list
            .connect_server_connect(glib::clone!(@weak state => move |server| {
                crate::terminal_launcher::launch_terminal(&server);
            }));

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

        // Search
        search_entry.connect_search_changed(
            glib::clone!(@weak self.server_list as list, @strong state => move |entry| {
                let query = entry.text().to_string().to_lowercase();
                let servers: Vec<Server> = state
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

        // Detail view signals
        self.server_detail
            .connect_connect_clicked(glib::clone!(@strong state => move |server| {
                crate::terminal_launcher::launch_terminal(&server);
            }));

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

        controller.connect_key_pressed(
            glib::clone!(@strong state, @strong server_list, @strong server_detail =>
                move |_, key, _, modifier| {
                if modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    match key {
                        gtk4::gdk::Key::n | gtk4::gdk::Key::N => {
                            // Add server shortcut handled by action
                            glib::Propagation::Proceed
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
                                dialogs::show_edit_server_dialog(
                                    &server_list.widget().ancestor(adw::ApplicationWindow::static_type())
                                        .and_downcast::<adw::ApplicationWindow>()
                                        .unwrap(),
                                    &server,
                                    glib::clone!(@strong state => move |updated| {
                                        state.borrow_mut().update_server(updated);
                                    }),
                                );
                            }
                            glib::Propagation::Stop
                        }
                        gtk4::gdk::Key::Delete | gtk4::gdk::Key::KP_Delete => {
                            // Delete selected server
                            if let Some(server) = state.borrow().get_selected_server() {
                                dialogs::show_confirm_delete_dialog(
                                    &server_list.widget().ancestor(adw::ApplicationWindow::static_type())
                                        .and_downcast::<adw::ApplicationWindow>()
                                        .unwrap(),
                                    &server.name,
                                    glib::clone!(@strong state => move || {
                                        state.borrow_mut().delete_server(&server.id);
                                        server_detail.set_server(None);
                                    }),
                                );
                            }
                            glib::Propagation::Stop
                        }
                        _ => glib::Propagation::Proceed,
                    }
                } else if key == gtk4::gdk::Key::F2 {
                    // Edit on F2
                    if let Some(server) = state.borrow().get_selected_server() {
                        dialogs::show_edit_server_dialog(
                            &server_list.widget().ancestor(adw::ApplicationWindow::static_type())
                                .and_downcast::<adw::ApplicationWindow>()
                                .unwrap(),
                            &server,
                            glib::clone!(@strong state => move |updated| {
                                state.borrow_mut().update_server(updated);
                            }),
                        );
                    }
                    glib::Propagation::Stop
                } else if key == gtk4::gdk::Key::Delete || key == gtk4::gdk::Key::KP_Delete {
                    // Delete key without modifier
                    if let Some(server) = state.borrow().get_selected_server() {
                        dialogs::show_confirm_delete_dialog(
                            &server_list.widget().ancestor(adw::ApplicationWindow::static_type())
                                .and_downcast::<adw::ApplicationWindow>()
                                .unwrap(),
                            &server.name,
                            glib::clone!(@strong state => move || {
                                state.borrow_mut().delete_server(&server.id);
                                server_detail.set_server(None);
                            }),
                        );
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }),
        );

        self.window.add_controller(controller);
    }

    fn load_data(&self) {
        // Load groups into sidebar
        let groups = self.state.borrow().get_groups();
        self.sidebar.set_groups(groups);

        // Load servers
        let servers = self.state.borrow().get_servers();
        self.server_list.set_servers(servers);
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn widget(&self) -> &adw::ApplicationWindow {
        &self.window
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
        }
    }
}
