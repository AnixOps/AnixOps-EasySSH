use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

use crate::models::{Server, ServerGroup};
use crate::views::{EmptyView, ServerDetailView, ServerListView};

pub struct EasySSHApp {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
    core_state: Arc<Mutex<easyssh_core::AppState>>,
}

impl EasySSHApp {
    pub fn new(app: &adw::Application) -> Self {
        // Initialize Rust core
        let core_state = Arc::new(Mutex::new(easyssh_core::AppState::new()));

        // Initialize database
        {
            let state = core_state.lock().unwrap();
            if let Err(e) = easyssh_core::init_database(&state) {
                tracing::error!("Failed to initialize database: {}", e);
            }
        }

        // Create header bar
        let header = adw::HeaderBar::new();

        // Add server button
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some("Add Server"));
        header.pack_start(&add_button);

        // Search entry
        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search servers..."));
        header.set_title_widget(Some(&search_entry));

        // Menu button
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        header.pack_end(&menu_button);

        // Create sidebar (server list)
        let server_list = ServerListView::new();
        let sidebar = gtk4::ScrolledWindow::new();
        sidebar.set_child(Some(&server_list.widget()));
        sidebar.set_width_request(280);

        // Create content stack
        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

        // Empty state
        let empty_view = EmptyView::new();
        stack.add_named(&empty_view.widget(), Some("empty"));

        // Server detail
        let detail_view = ServerDetailView::new(core_state.clone());
        stack.add_named(&detail_view.widget(), Some("detail"));

        // Paned layout (sidebar + content)
        let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&stack));
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(false);

        // Toolbar view for modern GNOME style
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&paned));

        // Breakpoint for responsive design
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            600.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&paned, "collapsed", &true.to_value());

        // Main window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("EasySSH")
            .default_width(900)
            .default_height(600)
            .content(&toolbar_view)
            .build();

        window.add_breakpoint(breakpoint);

        // Load servers
        Self::load_servers(&core_state, &server_list);

        Self {
            window,
            stack,
            core_state,
        }
    }

    fn load_servers(state: &Arc<Mutex<easyssh_core::AppState>>, view: &ServerListView) {
        let state = state.lock().unwrap();
        match easyssh_core::get_servers(&state) {
            Ok(servers) => {
                let servers: Vec<Server> = servers.into_iter().map(|s| s.into()).collect();
                view.set_servers(servers);
            }
            Err(e) => {
                tracing::error!("Failed to load servers: {}", e);
            }
        }

        match easyssh_core::get_groups(&state) {
            Ok(groups) => {
                let groups: Vec<ServerGroup> = groups.into_iter().map(|g| g.into()).collect();
                view.set_groups(groups);
            }
            Err(e) => {
                tracing::error!("Failed to load groups: {}", e);
            }
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
