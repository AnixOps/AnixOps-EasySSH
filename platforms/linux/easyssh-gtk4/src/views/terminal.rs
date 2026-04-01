use gtk4::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::app::AppViewModel;

pub struct TerminalView {
    widget: gtk4::Box,
    view_model: Arc<Mutex<AppViewModel>>,
    text_buffer: gtk4::TextBuffer,
    text_view: gtk4::TextView,
    command_entry: gtk4::Entry,
    receiver: RefCell<Option<UnboundedReceiver<String>>>,
    session_id: RefCell<Option<String>>,
    command_history: RefCell<Vec<String>>,
    history_index: RefCell<Option<usize>>,
}

impl TerminalView {
    pub fn new(view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_.add_css_class("terminal-view");

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        toolbar.set_margin_start(8);
        toolbar.set_margin_end(8);
        toolbar.set_margin_top(8);
        toolbar.set_margin_bottom(8);

        let title_label = gtk4::Label::new(Some("SSH Terminal"));
        title_label.add_css_class("title-3");

        let status_label = gtk4::Label::new(Some("● Connected"));
        status_label.add_css_class("status-connected");

        let disconnect_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        disconnect_btn.set_tooltip_text(Some("Disconnect"));
        disconnect_btn.add_css_class("destructive-action");

        let clear_btn = gtk4::Button::from_icon_name("edit-clear-all-symbolic");
        clear_btn.set_tooltip_text(Some("Clear Terminal (Ctrl+L)"));

        let interrupt_btn = gtk4::Button::from_icon_name("process-stop-symbolic");
        interrupt_btn.set_tooltip_text(Some("Interrupt (Ctrl+C)"));

        toolbar.append(&title_label);
        toolbar.append(&gtk4::Label::new(Some(" ")));
        toolbar.append(&status_label);

        let toolbar_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        toolbar_spacer.set_hexpand(true);
        toolbar.append(&toolbar_spacer);

        toolbar.append(&interrupt_btn);
        toolbar.append(&clear_btn);
        toolbar.append(&disconnect_btn);

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

        let prompt_label = gtk4::Label::new(Some("$"));
        prompt_label.add_css_class("status-connected");
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

        let view = Self {
            widget: box_,
            view_model,
            text_buffer,
            text_view,
            command_entry,
            receiver: RefCell::new(None),
            session_id: RefCell::new(None),
            command_history: RefCell::new(Vec::new()),
            history_index: RefCell::new(None),
        };

        // Connect signals
        view.setup_signals(&execute_btn, &clear_btn, &interrupt_btn, &disconnect_btn);
        view.setup_output_polling();

        view
    }

    fn setup_signals(
        &self,
        execute_btn: &gtk4::Button,
        clear_btn: &gtk4::Button,
        interrupt_btn: &gtk4::Button,
        _disconnect_btn: &gtk4::Button,
    ) {
        // Execute button
        execute_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.execute_command();
        }));

        // Clear button
        clear_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.clear_output();
        }));

        // Interrupt button (Ctrl+C equivalent)
        interrupt_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.interrupt_command();
        }));

        // Enter key on command entry
        self.command_entry
            .connect_activate(glib::clone!(@weak self as view => move |_| {
                view.execute_command();
            }));

        // History navigation
        self.command_entry.connect_key_pressed(
            glib::clone!(@weak self as view => move |_, key, _, _| {
                match key {
                    gtk4::gdk::Key::Up => {
                        view.navigate_history(true);
                        glib::Propagation::Stop
                    }
                    gtk4::gdk::Key::Down => {
                        view.navigate_history(false);
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            }),
        );
    }

    fn setup_output_polling(&self) {
        let text_buffer = self.text_buffer.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            // Poll receiver would go here, but we need access to self
            // This is simplified - in real implementation use a proper channel
            glib::ControlFlow::Continue
        });
    }

    pub fn start_stream(&self, receiver: UnboundedReceiver<String>) {
        self.receiver.replace(Some(receiver));

        // Start polling
        let weak_receiver = self.receiver.clone();
        let text_buffer = self.text_buffer.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            if let Some(receiver) = weak_receiver.borrow_mut().as_mut() {
                let mut received_any = false;
                while let Ok(chunk) = receiver.try_recv() {
                    let end_iter = text_buffer.end_iter();
                    text_buffer.insert(&end_iter, &chunk);
                    received_any = true;
                }

                if received_any {
                    // Auto-scroll to bottom
                    if let Some(mark) = text_buffer.get_mark("scroll") {
                        text_buffer.move_mark(&mark, &text_buffer.end_iter());
                    } else {
                        text_buffer.create_mark(Some("scroll"), &text_buffer.end_iter(), false);
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    pub fn set_session_id(&self, session_id: &str) {
        self.session_id.replace(Some(session_id.to_string()));
    }

    fn execute_command(&self) {
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

        // Display command in terminal
        let end_iter = self.text_buffer.end_iter();
        self.text_buffer
            .insert(&end_iter, &format!("$ {}\n", command));

        // Send command
        if let Some(ref session_id) = *self.session_id.borrow() {
            let line = format!("{}\n", command);
            let vm = self.view_model.lock().unwrap();
            if let Err(e) = vm.write_shell_input(session_id, line.as_bytes()) {
                let end_iter = self.text_buffer.end_iter();
                self.text_buffer
                    .insert(&end_iter, &format!("Error: {}\n", e));
            }
        }

        self.command_entry.set_text("");
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

    fn interrupt_command(&self) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            let vm = self.view_model.lock().unwrap();

            // Send Ctrl+C (0x03)
            let _ = vm.write_shell_input(session_id, &[0x03]);

            // Also try interrupt method
            let _ = vm.interrupt_command(session_id);

            let end_iter = self.text_buffer.end_iter();
            self.text_buffer.insert(&end_iter, "^C\n");
        }
    }

    fn disconnect(&self) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            let vm = self.view_model.lock().unwrap();
            let _ = vm.disconnect(session_id);
        }
        self.receiver.replace(None);
        self.session_id.replace(None);
    }

    pub fn append_output(&self, text: &str) {
        let end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&end_iter, text);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

use gtk4::glib;
use gtk4::prelude::WidgetExt;
