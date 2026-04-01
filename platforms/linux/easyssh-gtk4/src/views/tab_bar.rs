use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::app::AppViewModel;
use crate::models::Server;

/// Represents a single session tab with its own SSH connection
#[derive(Clone, Debug)]
pub struct SessionTab {
    pub session_id: String,
    pub server_id: String,
    pub title: String,
    pub server: Server,
    pub connected: bool,
    pub terminal_content: String,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
}

impl SessionTab {
    pub fn new(server: Server) -> Self {
        let session_id = Uuid::new_v4().to_string();
        let title = format!("{}@{}", server.username, server.host);

        Self {
            session_id,
            server_id: server.id.clone(),
            title,
            server,
            connected: false,
            terminal_content: String::new(),
            command_history: Vec::new(),
            history_index: None,
        }
    }
}

/// Tab bar widget with Chrome/VS Code style tabs
pub struct TabBar {
    widget: gtk4::Box,
    tabs: RefCell<Vec<SessionTab>>,
    active_tab_id: RefCell<Option<String>>,
    tab_buttons: RefCell<HashMap<String, gtk4::Button>>,
    tab_box: gtk4::Box,
    new_tab_button: gtk4::Button,

    // Callbacks
    on_tab_selected: RefCell<Option<Box<dyn Fn(&str) + 'static>>>,
    on_tab_closed: RefCell<Option<Box<dyn Fn(&str) + 'static>>>,
    on_new_tab: RefCell<Option<Box<dyn Fn() + 'static>>>,
    on_tab_reordered: RefCell<Option<Box<dyn Fn(Vec<String>) + 'static>>>,
}

impl TabBar {
    pub fn new() -> Self {
        // Main container - horizontal box for tabs
        let widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        widget.add_css_class("tab-bar");

        // Tab container with scrolling
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
        scrolled.set_hexpand(true);

        let tab_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
        tab_box.add_css_class("tab-box");
        scrolled.set_child(Some(&tab_box));

        // New tab button
        let new_tab_button = gtk4::Button::from_icon_name("list-add-symbolic");
        new_tab_button.set_tooltip_text(Some("New Tab (Ctrl+T)"));
        new_tab_button.add_css_class("new-tab-button");
        new_tab_button.set_margin_start(4);
        new_tab_button.set_margin_end(4);

        // Tab overflow menu button
        let overflow_button = gtk4::MenuButton::new();
        overflow_button.set_icon_name("pan-down-symbolic");
        overflow_button.set_tooltip_text(Some("More tabs"));
        overflow_button.add_css_class("tab-overflow-button");

        // Assemble
        widget.append(&scrolled);
        widget.append(&new_tab_button);
        widget.append(&overflow_button);

        let bar = Self {
            widget,
            tabs: RefCell::new(Vec::new()),
            active_tab_id: RefCell::new(None),
            tab_buttons: RefCell::new(HashMap::new()),
            tab_box,
            new_tab_button,
            on_tab_selected: RefCell::new(None),
            on_tab_closed: RefCell::new(None),
            on_new_tab: RefCell::new(None),
            on_tab_reordered: RefCell::new(None),
        };

        bar.setup_signals();
        bar.setup_css();

        bar
    }

    fn setup_css(&self) {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(
            r#"
            .tab-bar {
                background: @headerbar_shade_color;
                border-bottom: 1px solid @borders;
                min-height: 38px;
            }

            .tab-box {
                padding: 4px 4px 0 4px;
            }

            .tab-button {
                background: @headerbar_shade_color;
                border-radius: 8px 8px 0 0;
                border: none;
                border-bottom: 2px solid transparent;
                padding: 8px 12px;
                margin: 0 2px;
                min-width: 120px;
                transition: all 200ms ease;
            }

            .tab-button:hover {
                background: alpha(@theme_fg_color, 0.1);
            }

            .tab-button.active {
                background: @theme_base_color;
                border-bottom-color: @accent_color;
            }

            .tab-button .tab-close-button {
                opacity: 0;
                transition: opacity 200ms ease;
                border-radius: 4px;
                padding: 2px;
                min-width: 16px;
                min-height: 16px;
            }

            .tab-button:hover .tab-close-button,
            .tab-button.active .tab-close-button {
                opacity: 1;
            }

            .tab-close-button:hover {
                background: alpha(@error_color, 0.15);
            }

            .new-tab-button {
                border-radius: 4px;
                padding: 6px;
                min-width: 24px;
                min-height: 24px;
            }

            .tab-drag-source {
                opacity: 0.5;
            }

            .tab-drag-target {
                background: alpha(@accent_color, 0.1);
            }

            .tab-indicator {
                font-size: 8px;
                margin-right: 4px;
            }

            .tab-indicator.connected {
                color: @success_color;
            }

            .tab-indicator.disconnected {
                color: @error_color;
            }
            "#,
        );

        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not get display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn setup_signals(&self) {
        // New tab button
        self.new_tab_button
            .connect_clicked(glib::clone!(@weak self as bar => move |_| {
                if let Some(ref callback) = *bar.on_new_tab.borrow() {
                    callback();
                }
            }));
    }

    /// Add a new tab
    pub fn add_tab(&self, tab: SessionTab) -> String {
        let session_id = tab.session_id.clone();

        // Store tab data
        self.tabs.borrow_mut().push(tab.clone());

        // Create tab button
        let tab_button = self.create_tab_button(&tab);
        self.tab_buttons
            .borrow_mut()
            .insert(session_id.clone(), tab_button.clone());
        self.tab_box.append(&tab_button);

        // Set as active
        self.set_active_tab(&session_id);

        session_id
    }

    /// Create a tab button widget
    fn create_tab_button(&self, tab: &SessionTab) -> gtk4::Button {
        let button = gtk4::Button::new();
        button.add_css_class("tab-button");

        // Horizontal box for tab content
        let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

        // Connection indicator
        let indicator = gtk4::Label::new(Some("●"));
        indicator.add_css_class("tab-indicator");
        if tab.connected {
            indicator.add_css_class("connected");
        } else {
            indicator.add_css_class("disconnected");
        }

        // Tab title
        let title_label = gtk4::Label::new(Some(&tab.title));
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(20);
        title_label.set_hexpand(true);
        title_label.set_halign(gtk4::Align::Start);

        // Close button
        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("tab-close-button");
        close_btn.add_css_class("flat");
        close_btn.set_tooltip_text(Some("Close tab (Middle-click)"));
        close_btn.set_has_frame(false);

        hbox.append(&indicator);
        hbox.append(&title_label);
        hbox.append(&close_btn);

        button.set_child(Some(&hbox));

        // Tab click handler
        let session_id = tab.session_id.clone();
        let session_id_for_close = session_id.clone();
        let session_id_for_middle = session_id.clone();

        // Left click to select
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);
        gesture.connect_released(
            glib::clone!(@weak self as bar, @weak button => move |_, _, _, _| {
                bar.set_active_tab(&session_id);
                if let Some(ref callback) = *bar.on_tab_selected.borrow() {
                    callback(&session_id);
                }
            }),
        );
        button.add_controller(gesture);

        // Middle click to close
        let gesture_middle = gtk4::GestureClick::new();
        gesture_middle.set_button(gtk4::gdk::BUTTON_MIDDLE);
        gesture_middle.connect_released(glib::clone!(@weak self as bar => move |_, _, _, _| {
            bar.close_tab(&session_id_for_middle);
        }));
        button.add_controller(gesture_middle);

        // Close button - use GestureClick to avoid signal propagation issues
        let close_gesture = gtk4::GestureClick::new();
        close_gesture.set_button(gtk4::gdk::BUTTON_PRIMARY);
        close_gesture.connect_released(
            glib::clone!(@weak self as bar, @weak button => move |_, _, _, _| {
                // Don't select the tab when closing
                bar.close_tab(&session_id_for_close);
                // Prevent further handling
                glib::signal::signal_stop_emission_by_name(button.as_ref(), "clicked");
            }),
        );
        close_btn.add_controller(close_gesture);

        // Drag and drop for reordering
        self.setup_drag_drop(&button, &tab.session_id);

        button
    }

    fn setup_drag_drop(&self, button: &gtk4::Button, session_id: &str) {
        let session_id_drag = session_id.to_string();
        let session_id_drop = session_id.to_string();

        // Drag source - using string content
        let drag_content =
            gtk4::gdk::ContentProvider::from_value(glib::Value::from(&session_id_drag));
        let drag_source = gtk4::DragSource::builder()
            .content(&drag_content)
            .actions(gtk4::gdk::DragAction::MOVE)
            .build();

        drag_source.connect_drag_begin(glib::clone!(@weak button => move |_, _| {
            button.add_css_class("tab-drag-source");
        }));

        drag_source.connect_drag_end(glib::clone!(@weak button => move |_, _, _| {
            button.remove_css_class("tab-drag-source");
        }));

        button.add_controller(drag_source);

        // Drop target
        let drop_target = gtk4::DropTarget::new(glib::Type::STRING, gtk4::gdk::DragAction::MOVE);

        drop_target.connect_enter(glib::clone!(@weak button => move |_, _, _| {
            button.add_css_class("tab-drag-target");
            gtk4::gdk::DragAction::MOVE
        }));

        drop_target.connect_leave(glib::clone!(@weak button => move |_| {
            button.remove_css_class("tab-drag-target");
        }));

        drop_target.connect_drop(
            glib::clone!(@weak self as bar, @weak button => move |_, value, _, _| {
                button.remove_css_class("tab-drag-target");

                if let Ok(dragged_id) = value.get::<String>() {
                    let target_id = session_id_drop.clone();
                    if dragged_id != target_id {
                        bar.reorder_tabs(&dragged_id, &target_id);
                    }
                    true
                } else {
                    false
                }
            }),
        );

        button.add_controller(drop_target);
    }

    fn reorder_tabs(&self, dragged_id: &str, target_id: &str) {
        let mut tabs = self.tabs.borrow_mut();

        // Find positions
        let dragged_pos = tabs.iter().position(|t| t.session_id == dragged_id);
        let target_pos = tabs.iter().position(|t| t.session_id == target_id);

        if let (Some(from), Some(to)) = (dragged_pos, target_pos) {
            // Remove from old position and insert at new position
            let tab = tabs.remove(from);
            let new_pos = if from < to { to } else { to };
            tabs.insert(new_pos, tab);

            // Rebuild tab box
            drop(tabs);
            self.rebuild_tab_box();

            // Notify callback
            if let Some(ref callback) = *self.on_tab_reordered.borrow() {
                let order: Vec<String> = self
                    .tabs
                    .borrow()
                    .iter()
                    .map(|t| t.session_id.clone())
                    .collect();
                callback(order);
            }
        }
    }

    fn rebuild_tab_box(&self) {
        // Remove all children
        while let Some(child) = self.tab_box.first_child() {
            self.tab_box.remove(&child);
        }

        // Re-add all tabs
        let tabs = self.tabs.borrow();
        let buttons = self.tab_buttons.borrow();

        for tab in tabs.iter() {
            if let Some(button) = buttons.get(&tab.session_id) {
                self.tab_box.append(button);
            }
        }
    }

    /// Close a tab
    pub fn close_tab(&self, session_id: &str) {
        // Remove from tabs list
        {
            let mut tabs = self.tabs.borrow_mut();
            let pos = tabs.iter().position(|t| t.session_id == session_id);
            if let Some(idx) = pos {
                tabs.remove(idx);
            }
        }

        // Remove button
        if let Some(button) = self.tab_buttons.borrow_mut().remove(session_id) {
            self.tab_box.remove(&button);
        }

        // Update active tab if needed
        let was_active = self.active_tab_id.borrow().as_deref() == Some(session_id);
        if was_active {
            // Select another tab
            let tabs = self.tabs.borrow();
            if let Some(first_tab) = tabs.first() {
                self.set_active_tab(&first_tab.session_id);
            } else {
                self.active_tab_id.replace(None);
            }
        }

        // Notify callback
        if let Some(ref callback) = *self.on_tab_closed.borrow() {
            callback(session_id);
        }
    }

    /// Set active tab
    pub fn set_active_tab(&self, session_id: &str) {
        // Remove active class from previous
        if let Some(old_id) = self.active_tab_id.borrow().as_ref() {
            if let Some(button) = self.tab_buttons.borrow().get(old_id) {
                button.remove_css_class("active");
            }
        }

        // Add active class to new
        if let Some(button) = self.tab_buttons.borrow().get(session_id) {
            button.add_css_class("active");
            self.active_tab_id.replace(Some(session_id.to_string()));
        }
    }

    /// Get active tab ID
    pub fn active_tab(&self) -> Option<String> {
        self.active_tab_id.borrow().clone()
    }

    /// Get tab by session ID
    pub fn get_tab(&self, session_id: &str) -> Option<SessionTab> {
        self.tabs
            .borrow()
            .iter()
            .find(|t| t.session_id == session_id)
            .cloned()
    }

    /// Get all tabs
    pub fn get_all_tabs(&self) -> Vec<SessionTab> {
        self.tabs.borrow().clone()
    }

    /// Update tab terminal content
    pub fn update_tab_content(&self, session_id: &str, content: &str) {
        let mut tabs = self.tabs.borrow_mut();
        if let Some(tab) = tabs.iter_mut().find(|t| t.session_id == session_id) {
            tab.terminal_content = content.to_string();
        }
    }

    /// Update tab connection status
    pub fn update_tab_status(&self, session_id: &str, connected: bool) {
        let mut tabs = self.tabs.borrow_mut();
        if let Some(tab) = tabs.iter_mut().find(|t| t.session_id == session_id) {
            tab.connected = connected;
        }

        // Update indicator in UI
        if let Some(button) = self.tab_buttons.borrow().get(session_id) {
            if let Some(child) = button.child() {
                if let Some(hbox) = child.downcast_ref::<gtk4::Box>() {
                    if let Some(first_child) = hbox.first_child() {
                        if let Some(indicator) = first_child.downcast_ref::<gtk4::Label>() {
                            if connected {
                                indicator.remove_css_class("disconnected");
                                indicator.add_css_class("connected");
                            } else {
                                indicator.remove_css_class("connected");
                                indicator.add_css_class("disconnected");
                            }
                        }
                    }
                }
            }
        }
    }

    /// Switch to tab by index (for Ctrl+1-9)
    pub fn switch_to_tab_index(&self, index: usize) -> Option<String> {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.get(index) {
            let session_id = tab.session_id.clone();
            drop(tabs);
            self.set_active_tab(&session_id);
            if let Some(ref callback) = *self.on_tab_selected.borrow() {
                callback(&session_id);
            }
            Some(session_id)
        } else {
            None
        }
    }

    /// Check if has tabs
    pub fn has_tabs(&self) -> bool {
        !self.tabs.borrow().is_empty()
    }

    /// Get tab count
    pub fn tab_count(&self) -> usize {
        self.tabs.borrow().len()
    }

    /// Connect tab selected callback
    pub fn connect_tab_selected<F>(&self, callback: F)
    where
        F: Fn(&str) + 'static,
    {
        self.on_tab_selected.replace(Some(Box::new(callback)));
    }

    /// Connect tab closed callback
    pub fn connect_tab_closed<F>(&self, callback: F)
    where
        F: Fn(&str) + 'static,
    {
        self.on_tab_closed.replace(Some(Box::new(callback)));
    }

    /// Connect new tab callback
    pub fn connect_new_tab<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.on_new_tab.replace(Some(Box::new(callback)));
    }

    /// Connect tab reordered callback
    pub fn connect_tab_reordered<F>(&self, callback: F)
    where
        F: Fn(Vec<String>) + 'static,
    {
        self.on_tab_reordered.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

/// Multi-session terminal container with tab bar
pub struct MultiSessionTerminal {
    widget: gtk4::Box,
    tab_bar: TabBar,
    terminal_stack: gtk4::Stack,
    terminals: RefCell<HashMap<String, TerminalSession>>,
    view_model: Arc<Mutex<AppViewModel>>,
}

/// Individual terminal session widget
pub struct TerminalSession {
    session_id: String,
    widget: gtk4::Box,
    text_buffer: gtk4::TextBuffer,
    text_view: gtk4::TextView,
    command_entry: gtk4::Entry,
    receiver: RefCell<Option<UnboundedReceiver<String>>>,
    command_history: RefCell<Vec<String>>,
    history_index: RefCell<Option<usize>>,
}

impl TerminalSession {
    pub fn new(session_id: &str, view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_.add_css_class("terminal-session");

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        toolbar.set_margin_start(8);
        toolbar.set_margin_end(8);
        toolbar.set_margin_top(8);
        toolbar.set_margin_bottom(8);

        let status_label = gtk4::Label::new(Some("● Connected"));
        status_label.add_css_class("status-connected");

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let clear_btn = gtk4::Button::from_icon_name("edit-clear-all-symbolic");
        clear_btn.set_tooltip_text(Some("Clear Terminal (Ctrl+L)"));

        let interrupt_btn = gtk4::Button::from_icon_name("process-stop-symbolic");
        interrupt_btn.set_tooltip_text(Some("Interrupt (Ctrl+C)"));

        toolbar.append(&status_label);
        toolbar.append(&spacer);
        toolbar.append(&interrupt_btn);
        toolbar.append(&clear_btn);

        // Terminal output
        let text_buffer = gtk4::TextBuffer::new(None);
        let text_view = gtk4::TextView::with_buffer(&text_buffer);
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(gtk4::WrapMode::WordChar);
        text_view.add_css_class("terminal-output");
        text_view.set_monospace(true);

        let scrolled_window = gtk4::ScrolledWindow::new();
        scrolled_window.set_child(Some(&text_view));
        scrolled_window.set_vexpand(true);
        scrolled_window.set_hscrollbar_policy(gtk4::PolicyType::Never);

        // Command input
        let input_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        input_box.set_margin_start(8);
        input_box.set_margin_end(8);
        input_box.set_margin_bottom(8);

        let prompt_label = gtk4::Label::new(Some("❯"));
        prompt_label.set_markup("<span foreground='#4ade80'>❯</span>");

        let command_entry = gtk4::Entry::new();
        command_entry.set_hexpand(true);
        command_entry.set_placeholder_text(Some("Enter command..."));

        let execute_btn = gtk4::Button::with_label("Execute");
        execute_btn.add_css_class("suggested-action");

        input_box.append(&prompt_label);
        input_box.append(&command_entry);
        input_box.append(&execute_btn);

        box_.append(&toolbar);
        box_.append(&scrolled_window);
        box_.append(&input_box);

        let session = Self {
            session_id: session_id.to_string(),
            widget: box_,
            text_buffer,
            text_view,
            command_entry,
            receiver: RefCell::new(None),
            command_history: RefCell::new(Vec::new()),
            history_index: RefCell::new(None),
        };

        session.setup_signals(&execute_btn, &clear_btn, &interrupt_btn, view_model);
        session
    }

    fn setup_signals(
        &self,
        execute_btn: &gtk4::Button,
        clear_btn: &gtk4::Button,
        interrupt_btn: &gtk4::Button,
        view_model: Arc<Mutex<AppViewModel>>,
    ) {
        // Execute button
        execute_btn.connect_clicked(glib::clone!(@weak self as session => move |_| {
            session.execute_command(view_model.clone());
        }));

        // Clear button
        clear_btn.connect_clicked(glib::clone!(@weak self as session => move |_| {
            session.clear_output();
        }));

        // Interrupt button
        let vm_interrupt = view_model.clone();
        interrupt_btn.connect_clicked(glib::clone!(@weak self as session => move |_| {
            session.interrupt_command(vm_interrupt.clone());
        }));

        // Enter key
        self.command_entry
            .connect_activate(glib::clone!(@weak self as session => move |_| {
                session.execute_command(view_model.clone());
            }));

        // History navigation
        self.command_entry.connect_key_pressed(
            glib::clone!(@weak self as session => move |_, key, _, _| {
                match key {
                    gtk4::gdk::Key::Up => {
                        session.navigate_history(true);
                        glib::Propagation::Stop
                    }
                    gtk4::gdk::Key::Down => {
                        session.navigate_history(false);
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            }),
        );
    }

    pub fn start_stream(&self, receiver: UnboundedReceiver<String>) {
        self.receiver.replace(Some(receiver));

        let weak_receiver = self.receiver.clone();
        let text_buffer = self.text_buffer.clone();
        let text_view = self.text_view.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            if let Some(receiver) = weak_receiver.borrow_mut().as_mut() {
                let mut received_any = false;
                while let Ok(chunk) = receiver.try_recv() {
                    let end_iter = text_buffer.end_iter();
                    text_buffer.insert(&end_iter, &chunk);
                    received_any = true;
                }

                if received_any {
                    // Auto-scroll
                    if let Some(mark) = text_buffer.get_mark("scroll") {
                        text_buffer.move_mark(&mark, &text_buffer.end_iter());
                    } else {
                        text_buffer.create_mark(Some("scroll"), &text_buffer.end_iter(), false);
                    }

                    // Scroll view
                    if let Some(adj) = text_view.vadjustment() {
                        adj.set_value(adj.upper() - adj.page_size());
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    fn execute_command(&self, view_model: Arc<Mutex<AppViewModel>>) {
        let command = self.command_entry.text().to_string();
        if command.is_empty() {
            return;
        }

        // Add to history
        let mut history = self.command_history.borrow_mut();
        if !history.contains(&command) {
            history.push(command.clone());
            if history.len() > 100 {
                history.remove(0);
            }
        }
        self.history_index.replace(None);
        drop(history);

        // Display command
        let end_iter = self.text_buffer.end_iter();
        self.text_buffer
            .insert(&end_iter, &format!("$ {}\n", command));

        // Send command
        let line = format!("{}\n", command);
        let sid = self.session_id.clone();
        let vm = view_model.lock().unwrap();
        if let Err(e) = vm.write_shell_input(&sid, line.as_bytes()) {
            let end_iter = self.text_buffer.end_iter();
            self.text_buffer
                .insert(&end_iter, &format!("Error: {}\n", e));
        }

        self.command_entry.set_text("");
    }

    fn interrupt_command(&self, view_model: Arc<Mutex<AppViewModel>>) {
        let sid = self.session_id.clone();
        let vm = view_model.lock().unwrap();

        // Send Ctrl+C
        let _ = vm.write_shell_input(&sid, &[0x03]);
        let _ = vm.interrupt_command(&sid);

        let end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&end_iter, "^C\n");
    }

    fn navigate_history(&self, up: bool) {
        let history = self.command_history.borrow();
        if history.is_empty() {
            return;
        }

        let mut idx = self.history_index.borrow_mut();
        if up {
            *idx = match *idx {
                None => Some(history.len().saturating_sub(1)),
                Some(i) => Some(i.saturating_sub(1)),
            };
        } else {
            *idx = match *idx {
                None => return,
                Some(i) => {
                    if i >= history.len() - 1 {
                        self.command_entry.set_text("");
                        None
                    } else {
                        Some(i + 1)
                    }
                }
            };
        }

        if let Some(i) = *idx {
            if let Some(cmd) = history.get(i) {
                self.command_entry.set_text(cmd);
                self.command_entry.set_position(-1);
            }
        }
    }

    fn clear_output(&self) {
        let start = self.text_buffer.start_iter();
        let end = self.text_buffer.end_iter();
        self.text_buffer.delete(&start, &end);
    }

    pub fn append_output(&self, text: &str) {
        let end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&end_iter, text);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

impl MultiSessionTerminal {
    pub fn new(view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Tab bar
        let tab_bar = TabBar::new();

        // Terminal stack
        let terminal_stack = gtk4::Stack::new();
        terminal_stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        terminal_stack.set_hexpand(true);
        terminal_stack.set_vexpand(true);

        widget.append(tab_bar.widget());
        widget.append(&terminal_stack);

        Self {
            widget,
            tab_bar,
            terminal_stack,
            terminals: RefCell::new(HashMap::new()),
            view_model,
        }
    }

    /// Create a new session tab
    pub fn create_session(&self, server: Server) -> anyhow::Result<String> {
        let session_id = Uuid::new_v4().to_string();

        // Create tab
        let tab = SessionTab::new(server.clone());
        self.tab_bar.add_tab(tab.clone());

        // Create terminal session
        let terminal = TerminalSession::new(&session_id, self.view_model.clone());

        // Add to stack
        self.terminal_stack
            .add_named(terminal.widget(), &session_id);

        // Store
        self.terminals
            .borrow_mut()
            .insert(session_id.clone(), terminal);

        // Show this session
        self.terminal_stack.set_visible_child_name(&session_id);

        Ok(session_id)
    }

    /// Switch to session
    pub fn switch_to_session(&self, session_id: &str) {
        self.tab_bar.set_active_tab(session_id);
        self.terminal_stack.set_visible_child_name(session_id);
    }

    /// Close session
    pub fn close_session(&self, session_id: &str) {
        // Remove terminal
        if let Some(terminal) = self.terminals.borrow_mut().remove(session_id) {
            self.terminal_stack.remove(terminal.widget());
        }

        // Remove tab
        self.tab_bar.close_tab(session_id);
    }

    /// Get terminal for session
    pub fn get_terminal(&self, session_id: &str) -> Option<std::cell::Ref<TerminalSession>> {
        // This is a simplified version - in real implementation
        // you'd need RefCell borrow handling
        None
    }

    /// Connect to tab selected
    pub fn connect_tab_selected<F>(&self, callback: F)
    where
        F: Fn(&str) + 'static,
    {
        self.tab_bar.connect_tab_selected(callback);
    }

    /// Connect to tab closed
    pub fn connect_tab_closed<F>(&self, callback: F)
    where
        F: Fn(&str) + 'static,
    {
        self.tab_bar.connect_tab_closed(callback);
    }

    /// Connect to new tab
    pub fn connect_new_tab<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.tab_bar.connect_new_tab(callback);
    }

    /// Switch to tab by index (Ctrl+1-9)
    pub fn switch_to_tab_index(&self, index: usize) -> Option<String> {
        self.tab_bar.switch_to_tab_index(index)
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn has_sessions(&self) -> bool {
        self.tab_bar.has_tabs()
    }

    pub fn active_session(&self) -> Option<String> {
        self.tab_bar.active_tab()
    }

    /// Get all tab session IDs in order
    pub fn get_all_tab_ids(&self) -> Vec<String> {
        self.tab_bar
            .get_all_tabs()
            .iter()
            .map(|t| t.session_id.clone())
            .collect()
    }
}

use gtk4::glib;
