use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;
use crate::app::AppViewModel;
use crate::models::Server;

pub struct AddServerDialog {
    dialog: adw::Dialog,
    view_model: Arc<Mutex<AppViewModel>>,
    name_entry: gtk4::Entry,
    host_entry: gtk4::Entry,
    port_entry: gtk4::Entry,
    username_entry: gtk4::Entry,
    password_entry: gtk4::PasswordEntry,
    auth_combo: gtk4::DropDown,
    save_switch: gtk4::Switch,
    save_callback: RefCell<Option<Box<dyn Fn()>>>,
    cancel_callback: RefCell<Option<Box<dyn Fn()>>>,
}

impl AddServerDialog {
    pub fn new(parent: &adw::ApplicationWindow, view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let dialog = adw::Dialog::builder()
            .title("Add Server")
            .content_width(420)
            .content_height(480)
            .build();

        // Header
        let header = adw::HeaderBar::new();

        // Toolbar view
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);

        // Content
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(16);
        content.set_margin_end(16);

        // Form
        let form = adw::PreferencesGroup::new();
        form.set_title("Server Details");

        // Name
        let name_row = adw::EntryRow::new();
        name_row.set_title("Name");
        name_row.set_placeholder_text(Some("My Server"));
        form.add(&name_row);

        // Host
        let host_row = adw::EntryRow::new();
        host_row.set_title("Host");
        host_row.set_placeholder_text(Some("192.168.1.100 or example.com"));
        form.add(&host_row);

        // Port
        let port_row = adw::SpinRow::new(
            gtk4::Adjustment::new(22.0, 1.0, 65535.0, 1.0, 10.0, 0.0),
            gtk4::SpinButtonUpdatePolicy::IfValid,
        );
        port_row.set_title("Port");
        form.add(&port_row);

        // Username
        let username_row = adw::EntryRow::new();
        username_row.set_title("Username");
        username_row.set_placeholder_text(Some("root"));
        form.add(&username_row);

        // Authentication type
        let auth_row = adw::ComboRow::new();
        let auth_model = gtk4::StringList::new(&["Password", "SSH Key", "SSH Agent"]);
        auth_row.set_model(Some(&auth_model));
        auth_row.set_title("Authentication");
        auth_row.set_selected(0);
        form.add(&auth_row);

        // Password
        let password_row = adw::PasswordEntryRow::new();
        password_row.set_title("Password");
        password_row.set_show_apply_button(false);
        form.add(&password_row);

        // Save password switch
        let save_row = adw::SwitchRow::new();
        save_row.set_title("Save Password");
        save_row.set_subtitle("Store in system keyring");
        form.add(&save_row);

        content.append(&form);

        // Buttons
        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(16);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.add_css_class("pill");

        let save_btn = gtk4::Button::with_label("Add Server");
        save_btn.add_css_class("suggested-action");
        save_btn.add_css_class("pill");

        button_box.append(&cancel_btn);
        button_box.append(&save_btn);
        content.append(&button_box);

        toolbar_view.set_content(Some(&content));
        dialog.set_child(Some(&toolbar_view));

        let dialog_obj = Self {
            dialog: dialog.clone(),
            view_model,
            name_entry: name_row.clone(),
            host_entry: host_row.clone(),
            port_entry: port_row.clone(),
            username_entry: username_row.clone(),
            password_entry: password_row.clone(),
            auth_combo: auth_row.clone(),
            save_switch: save_row.clone(),
            save_callback: RefCell::new(None),
            cancel_callback: RefCell::new(None),
        };

        // Connect signals
        save_btn.connect_clicked(glib::clone!(@weak dialog, @weak dialog_obj as this => move |_| {
            this.do_save();
        }));

        cancel_btn.connect_clicked(glib::clone!(@weak dialog, @weak dialog_obj as this => move |_| {
            dialog.close();
            if let Some(cb) = this.cancel_callback.take() {
                cb();
            }
        }));

        dialog.present(Some(parent));

        dialog_obj
    }

    fn do_save(&self) {
        let name = self.name_entry.text().to_string();
        let host = self.host_entry.text().to_string();
        let port: i64 = self.port_entry.text().parse().unwrap_or(22);
        let username = self.username_entry.text().to_string();
        let auth_type = match self.auth_combo.selected() {
            0 => "password",
            1 => "key",
            _ => "agent",
        };

        if name.is_empty() {
            self.show_error("Name is required");
            return;
        }
        if host.is_empty() {
            self.show_error("Host is required");
            return;
        }
        if username.is_empty() {
            self.show_error("Username is required");
            return;
        }

        let vm = self.view_model.lock().unwrap();
        match vm.add_server(&name, &host, port, &username, auth_type) {
            Ok(_) => {
                // Save password if requested
                if self.save_switch.is_active() {
                    let password = self.password_entry.text().to_string();
                    if !password.is_empty() {
                        // Get the server ID (we need to retrieve it)
                        if let Ok(servers) = vm.get_servers() {
                            if let Some(server) = servers.iter().find(|s| s.name == name && s.host == host) {
                                let _ = vm.save_password(&server.id, &password);
                            }
                        }
                    }
                }

                self.dialog.close();
                if let Some(cb) = self.save_callback.take() {
                    cb();
                }
            }
            Err(e) => {
                self.show_error(&format!("Failed to add: {}", e));
            }
        }
    }

    fn show_error(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        // In a real implementation, we'd need access to the ToastOverlay
        tracing::error!("{}", message);
    }

    pub fn connect_save<F: Fn() + 'static>(&self, callback: F) {
        self.save_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_cancel<F: Fn() + 'static>(&self, callback: F) {
        self.cancel_callback.replace(Some(Box::new(callback)));
    }
}

pub struct ConnectDialog {
    dialog: adw::Dialog,
    server: Server,
    view_model: Arc<Mutex<AppViewModel>>,
    password_entry: gtk4::PasswordEntry,
    save_switch: gtk4::Switch,
    saved_password_loaded: RefCell<bool>,
    connect_callback: RefCell<Option<Box<dyn Fn(Option<String>, bool)>>>,
    cancel_callback: RefCell<Option<Box<dyn Fn()>>>,
}

impl ConnectDialog {
    pub fn new(parent: &adw::ApplicationWindow, server: Server, view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let dialog = adw::Dialog::builder()
            .title(&format!("Connect to {}", server.name))
            .content_width(400)
            .content_height(320)
            .build();

        // Header
        let header = adw::HeaderBar::new();

        // Toolbar view
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);

        // Content
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(16);
        content.set_margin_end(16);

        // Server info card
        let info_card = adw::PreferencesGroup::new();
        info_card.set_title("Connection Details");

        let host_row = adw::ActionRow::new();
        host_row.set_title("Host");
        host_row.set_subtitle(&format!("{}@{}", server.username, server.host));
        info_card.add(&host_row);

        let port_row = adw::ActionRow::new();
        port_row.set_title("Port");
        port_row.set_subtitle(&server.port.to_string());
        info_card.add(&port_row);

        content.append(&info_card);

        // Password section
        let password_group = adw::PreferencesGroup::new();
        password_group.set_title("Authentication");

        let password_row = adw::PasswordEntryRow::new();
        password_row.set_title("Password");
        password_row.set_show_apply_button(false);
        password_group.add(&password_row);

        // Save password switch
        let save_row = adw::SwitchRow::new();
        save_row.set_title("Save Password");
        save_row.set_subtitle("Store in system keyring for future connections");
        password_group.add(&save_row);

        content.append(&password_group);

        // Buttons
        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(16);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.add_css_class("pill");

        let connect_btn = gtk4::Button::with_label("Connect");
        connect_btn.add_css_class("suggested-action");
        connect_btn.add_css_class("pill");

        button_box.append(&cancel_btn);
        button_box.append(&connect_btn);
        content.append(&button_box);

        toolbar_view.set_content(Some(&content));
        dialog.set_child(Some(&toolbar_view));

        let dialog_obj = Self {
            dialog: dialog.clone(),
            server,
            view_model,
            password_entry: password_row.clone(),
            save_switch: save_row.clone(),
            saved_password_loaded: RefCell::new(false),
            connect_callback: RefCell::new(None),
            cancel_callback: RefCell::new(None),
        };

        // Connect signals
        connect_btn.connect_clicked(glib::clone!(@weak dialog, @weak dialog_obj as this => move |_| {
            let password = this.password_entry.text().to_string();
            let password_opt = if password.is_empty() { None } else { Some(password) };
            let save = this.save_switch.is_active();

            dialog.close();
            if let Some(cb) = this.connect_callback.take() {
                cb(password_opt, save);
            }
        }));

        cancel_btn.connect_clicked(glib::clone!(@weak dialog, @weak dialog_obj as this => move |_| {
            dialog.close();
            if let Some(cb) = this.cancel_callback.take() {
                cb();
            }
        }));

        dialog.present(Some(parent));

        dialog_obj
    }

    pub fn set_saved_password(&self, password: &str) {
        self.password_entry.set_text(password);
        self.save_switch.set_active(true);
        *self.saved_password_loaded.borrow_mut() = true;
    }

    pub fn connect_connect<F: Fn(Option<String>, bool) + 'static>(&self, callback: F) {
        self.connect_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_cancel<F: Fn() + 'static>(&self, callback: F) {
        self.cancel_callback.replace(Some(Box::new(callback)));
    }
}
