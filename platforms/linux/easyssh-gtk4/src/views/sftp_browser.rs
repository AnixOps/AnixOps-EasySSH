use gtk4::glib;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;

use crate::app::{AppViewModel, SftpEntry};
use crate::models::Server;

pub struct SftpBrowserView {
    widget: gtk4::Box,
    view_model: Arc<AppViewModel>,
    server: RefCell<Option<Server>>,
    session_id: RefCell<Option<String>>,
    path_bar: gtk4::Entry,
    file_list: gtk4::ListView,
    status_label: gtk4::Label,
    breadcrumb_box: gtk4::Box,
    current_path: RefCell<String>,
    file_store: gtk4::gio::ListStore,
    navigate_callback: RefCell<Option<Box<dyn Fn(&str)>>>,
    error_callback: RefCell<Option<Box<dyn Fn(&str)>>>,
}

#[derive(Clone, Debug)]
pub struct SftpFileItem {
    pub name: String,
    pub path: String,
    pub file_type: SftpFileType,
    pub size: i64,
    pub modified_time: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SftpFileType {
    File,
    Directory,
    Symlink,
    Unknown,
}

impl SftpFileItem {
    pub fn new(name: String, path: String, file_type: SftpFileType, size: i64, mtime: i64) -> Self {
        let modified_time = if mtime > 0 {
            let dt =
                chrono::DateTime::from_timestamp(mtime, 0).unwrap_or_else(|| chrono::Utc::now());
            dt.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "-".to_string()
        };

        Self {
            name,
            path,
            file_type,
            size,
            modified_time,
        }
    }

    pub fn size_display(&self) -> String {
        match self.file_type {
            SftpFileType::Directory => "-".to_string(),
            _ => format_size(self.size as u64),
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self.file_type {
            SftpFileType::Directory => "folder-symbolic",
            SftpFileType::Symlink => "emblem-symbolic-link",
            SftpFileType::File => "text-x-generic-symbolic",
            SftpFileType::Unknown => "unknown-file-type-symbolic",
        }
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

impl SftpBrowserView {
    pub fn new(view_model: Arc<AppViewModel>) -> Self {
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        main_box.set_vexpand(true);
        main_box.set_hexpand(true);

        // Toolbar
        let toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        toolbar.set_margin(12);

        let back_btn = gtk4::Button::from_icon_name("go-previous-symbolic");
        back_btn.set_tooltip_text(Some("Go Back"));
        let up_btn = gtk4::Button::from_icon_name("go-up-symbolic");
        up_btn.set_tooltip_text(Some("Go to Parent Directory"));
        let home_btn = gtk4::Button::from_icon_name("go-home-symbolic");
        home_btn.set_tooltip_text(Some("Go to Home"));
        let refresh_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_tooltip_text(Some("Refresh"));

        toolbar.append(&back_btn);
        toolbar.append(&up_btn);
        toolbar.append(&home_btn);
        toolbar.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        toolbar.append(&refresh_btn);

        let path_bar = gtk4::Entry::new();
        path_bar.set_hexpand(true);
        path_bar.set_placeholder_text(Some("Enter path..."));
        toolbar.append(&path_bar);

        let new_folder_btn = gtk4::Button::from_icon_name("folder-new-symbolic");
        new_folder_btn.set_tooltip_text(Some("New Folder"));
        let upload_btn = gtk4::Button::from_icon_name("document-send-symbolic");
        upload_btn.set_tooltip_text(Some("Upload File"));
        let download_btn = gtk4::Button::from_icon_name("document-save-symbolic");
        download_btn.set_tooltip_text(Some("Download"));
        let delete_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
        delete_btn.set_tooltip_text(Some("Delete"));
        delete_btn.add_css_class("destructive-action");

        toolbar.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
        toolbar.append(&new_folder_btn);
        toolbar.append(&upload_btn);
        toolbar.append(&download_btn);
        toolbar.append(&delete_btn);

        main_box.append(&toolbar);

        // Breadcrumb navigation
        let breadcrumb_scroll = gtk4::ScrolledWindow::new();
        breadcrumb_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
        breadcrumb_scroll.set_margin(12);

        let breadcrumb_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        breadcrumb_box.add_css_class("breadcrumb-bar");
        breadcrumb_scroll.set_child(Some(&breadcrumb_box));
        main_box.append(&breadcrumb_scroll);

        // File list using ListView
        let file_store = gtk4::gio::ListStore::new::<SftpFileObject>();
        let selection_model = gtk4::SingleSelection::new(Some(file_store.clone()));
        let file_list = gtk4::ListView::new(Some(selection_model), None::<&gtk4::ListItemFactory>);
        file_list.set_vexpand(true);
        file_list.set_hexpand(true);
        file_list.add_css_class("file-list");

        let list_scroll = gtk4::ScrolledWindow::new();
        list_scroll.set_child(Some(&file_list));
        list_scroll.set_vexpand(true);
        main_box.append(&list_scroll);

        // Status bar
        let status_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        status_bar.set_margin(12);

        let status_label = gtk4::Label::new(Some("Ready"));
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_hexpand(true);
        status_bar.append(&status_label);

        main_box.append(&status_bar);

        let view = Self {
            widget: main_box,
            view_model,
            server: RefCell::new(None),
            session_id: RefCell::new(None),
            path_bar,
            file_list,
            status_label,
            breadcrumb_box,
            current_path: RefCell::new("/".to_string()),
            file_store,
            navigate_callback: RefCell::new(None),
            error_callback: RefCell::new(None),
        };

        view.setup_signals(
            &back_btn,
            &up_btn,
            &home_btn,
            &refresh_btn,
            &new_folder_btn,
            &upload_btn,
            &download_btn,
            &delete_btn,
        );
        view.setup_drop_target();

        view
    }

    fn setup_signals(
        &self,
        back_btn: &gtk4::Button,
        up_btn: &gtk4::Button,
        home_btn: &gtk4::Button,
        refresh_btn: &gtk4::Button,
        new_folder_btn: &gtk4::Button,
        upload_btn: &gtk4::Button,
        download_btn: &gtk4::Button,
        delete_btn: &gtk4::Button,
    ) {
        back_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.show_message("Back navigation not yet implemented");
        }));

        up_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            let current = view.current_path.borrow().clone();
            if let Some(parent) = std::path::Path::new(&current).parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if !parent_str.is_empty() {
                    view.navigate_to(&parent_str);
                } else {
                    view.navigate_to("/");
                }
            }
        }));

        home_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.navigate_to("~");
        }));

        refresh_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            let current = view.current_path.borrow().clone();
            view.refresh_directory(&current);
        }));

        new_folder_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.show_new_folder_dialog();
        }));

        upload_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.show_upload_dialog();
        }));

        download_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.download_selected();
        }));

        delete_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.delete_selected();
        }));

        self.path_bar
            .connect_activate(glib::clone!(@weak self as view => move |entry| {
                let path = entry.text().to_string();
                view.navigate_to(&path);
            }));
    }

    fn setup_drop_target(&self) {
        let drop_target = gtk4::DropTarget::new(
            gtk4::gdk::FileList::static_type(),
            gtk4::gdk::DragAction::COPY,
        );

        let view = self as *const _;
        drop_target.connect_drop(move |_, value, _, _| {
            if let Ok(file_list) = value.get::<gtk4::gdk::FileList>() {
                for file in file_list.files() {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        unsafe { (*view).upload_file(&path_str) };
                    }
                }
                true
            } else {
                false
            }
        });

        self.file_list.add_controller(drop_target);
    }

    pub fn set_server(&self, server: Server, session_id: String) {
        self.server.replace(Some(server));
        self.session_id.replace(Some(session_id));
        self.init_sftp_session();
        self.navigate_to("~");
    }

    fn init_sftp_session(&self) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            self.view_model.init_sftp(session_id);
            self.show_message("SFTP session initialized");
        }
    }

    pub fn navigate_to(&self, path: &str) {
        let path = if path == "~" {
            "/home/".to_string()
        } else {
            path.to_string()
        };

        self.current_path.replace(path.clone());
        self.path_bar.set_text(&path);
        self.update_breadcrumb(&path);
        self.refresh_directory(&path);

        if let Some(ref cb) = *self.navigate_callback.borrow() {
            cb(&path);
        }
    }

    fn update_breadcrumb(&self, path: &str) {
        while let Some(child) = self.breadcrumb_box.first_child() {
            self.breadcrumb_box.remove(&child);
        }

        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let root_btn = gtk4::Button::from_icon_name("drive-harddisk-symbolic");
        root_btn.add_css_class("flat");
        root_btn.set_tooltip_text(Some("/"));
        root_btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
            view.navigate_to("/");
        }));
        self.breadcrumb_box.append(&root_btn);

        let mut current_path = String::from("/");
        for (i, part) in parts.iter().enumerate() {
            let sep = gtk4::Label::new(Some(" / "));
            sep.add_css_class("dim-label");
            self.breadcrumb_box.append(&sep);

            current_path.push_str(part);

            let btn = gtk4::Button::with_label(part);
            btn.add_css_class("flat");
            let path_clone = current_path.clone();
            btn.connect_clicked(glib::clone!(@weak self as view => move |_| {
                view.navigate_to(&path_clone);
            }));
            self.breadcrumb_box.append(&btn);

            if i < parts.len() - 1 {
                current_path.push('/');
            }
        }
    }

    fn refresh_directory(&self, path: &str) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            self.show_message("Loading...");

            let vm = self.view_model.clone();
            let sid = session_id.clone();
            let path = path.to_string();

            std::thread::spawn(glib::clone!(@weak self as view => move || {
                let vm = vm;

                if !vm.is_sftp_initialized(&sid) {
                    vm.init_sftp(&sid);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                match vm.sftp_list_dir(&sid, &path) {
                    Ok(entries) => {
                        let items: Vec<SftpFileItem> = entries.into_iter()
                            .filter(|e| e.name != "." && e.name != "..")
                            .map(|e| {
                                let file_type = match e.file_type.as_str() {
                                    "directory" => SftpFileType::Directory,
                                    "symlink" => SftpFileType::Symlink,
                                    _ => SftpFileType::File,
                                };
                                SftpFileItem::new(e.name, e.path, file_type, e.size, e.mtime)
                            })
                            .collect();

                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                            view.update_file_list(items);
                            view.show_message(&format!("{} items", view.file_store.n_items()));
                        }));
                    }
                    Err(e) => {
                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                            view.show_error(&format!("Failed to list directory: {}", e));
                        }));
                    }
                }
            }));
        }
    }

    fn update_file_list(&self, items: Vec<SftpFileItem>) {
        self.file_store.remove_all();
        for item in items {
            self.file_store.append(&SftpFileObject::new(item));
        }
    }

    fn show_new_folder_dialog(&self) {
        let dialog = adw::Dialog::builder()
            .title("New Folder")
            .content_width(360)
            .content_height(180)
            .build();

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.set_margin(16);

        let entry = adw::EntryRow::new();
        entry.set_title("Folder Name");
        entry.set_placeholder_text(Some("New Folder"));
        content.append(&entry);

        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin(8);

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.add_css_class("pill");

        let create_btn = gtk4::Button::with_label("Create");
        create_btn.add_css_class("suggested-action");
        create_btn.add_css_class("pill");

        button_box.append(&cancel_btn);
        button_box.append(&create_btn);
        content.append(&button_box);

        dialog.set_child(Some(&content));

        let weak_dialog = dialog.downgrade();
        cancel_btn.connect_clicked(move |_| {
            if let Some(d) = weak_dialog.upgrade() {
                d.close();
            }
        });

        let name_entry = entry.clone();
        create_btn.connect_clicked(glib::clone!(@weak self as view, @weak dialog => move |_| {
            let name = name_entry.text().to_string();
            if !name.is_empty() {
                view.create_folder(&name);
                dialog.close();
            }
        }));

        dialog.present(Some(
            &self.widget.root().and_downcast::<gtk4::Window>().unwrap(),
        ));
    }

    fn create_folder(&self, name: &str) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            let current = self.current_path.borrow().clone();
            let path = if current.ends_with('/') {
                format!("{}{}", current, name)
            } else {
                format!("{}/{}", current, name)
            };

            let vm = self.view_model.clone();
            let sid = session_id.clone();

            std::thread::spawn(glib::clone!(@weak self as view => move || {
                match vm.sftp_mkdir(&sid, &path) {
                    Ok(_) => {
                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                            view.show_message("Folder created");
                            view.refresh_directory(&view.current_path.borrow().clone());
                        }));
                    }
                    Err(e) => {
                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                            view.show_error(&format!("Failed to create folder: {}", e));
                        }));
                    }
                }
            }));
        }
    }

    fn show_upload_dialog(&self) {
        let file_dialog = gtk4::FileDialog::builder()
            .title("Select File to Upload")
            .build();

        file_dialog.open(
            Some(&self.widget.root().and_downcast::<gtk4::Window>().unwrap()),
            None::<&gtk4::gio::Cancellable>,
            glib::clone!(@weak self as view => move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        view.upload_file(&path_str);
                    }
                }
            }),
        );
    }

    fn upload_file(&self, local_path: &str) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            let current = self.current_path.borrow().clone();
            let file_name = std::path::Path::new(local_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("uploaded_file")
                .to_string();

            let remote_path = if current.ends_with('/') {
                format!("{}{}", current, file_name)
            } else {
                format!("{}/{}", current, file_name)
            };

            let local_path = local_path.to_string();
            let vm = self.view_model.clone();
            let sid = session_id.clone();

            self.show_message(&format!("Uploading {}...", file_name));

            std::thread::spawn(glib::clone!(@weak self as view => move || {
                match std::fs::read(&local_path) {
                    Ok(contents) => {
                        match vm.sftp_upload(&sid, &remote_path, &contents) {
                            Ok(_) => {
                                glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                    view.show_message(&format!("Uploaded {}", file_name));
                                    view.refresh_directory(&view.current_path.borrow().clone());
                                }));
                            }
                            Err(e) => {
                                glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                    view.show_error(&format!("Upload failed: {}", e));
                                }));
                            }
                        }
                    }
                    Err(e) => {
                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                            view.show_error(&format!("Failed to read file: {}", e));
                        }));
                    }
                }
            }));
        }
    }

    fn download_selected(&self) {
        let selection = self
            .file_list
            .model()
            .unwrap()
            .downcast_ref::<gtk4::SingleSelection>()
            .unwrap();

        if let Some(item) = selection.selected_item() {
            let file_obj = item.downcast::<SftpFileObject>().unwrap();
            let file_item = file_obj.file_item();

            if file_item.file_type == SftpFileType::File {
                let file_dialog = gtk4::FileDialog::builder()
                    .title("Save File")
                    .initial_name(&file_item.name)
                    .build();

                let remote_path = file_item.path.clone();
                let file_name = file_item.name.clone();
                let vm = self.view_model.clone();

                if let Some(ref session_id) = *self.session_id.borrow() {
                    let sid = session_id.clone();

                    file_dialog.save(
                        Some(&self.widget.root().and_downcast::<gtk4::Window>().unwrap()),
                        None::<&gtk4::gio::Cancellable>,
                        glib::clone!(@weak self as view => move |result| {
                            if let Ok(file) = result {
                                if let Some(local_path) = file.path() {
                                    let local_path_str = local_path.to_string_lossy().to_string();

                                    view.show_message(&format!("Downloading {}...", file_name));

                                    std::thread::spawn(glib::clone!(@weak view => move || {
                                        match vm.sftp_download(&sid, &remote_path) {
                                            Ok(contents) => {
                                                match std::fs::write(&local_path_str, &contents) {
                                                    Ok(_) => {
                                                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                                            view.show_message(&format!("Downloaded {}", file_name));
                                                        }));
                                                    }
                                                    Err(e) => {
                                                        glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                                            view.show_error(&format!("Failed to save file: {}", e));
                                                        }));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                                    view.show_error(&format!("Download failed: {}", e));
                                                }));
                                            }
                                        }
                                    }));
                                }
                            }
                        }),
                    );
                }
            }
        } else {
            self.show_message("No file selected");
        }
    }

    fn delete_selected(&self) {
        let selection = self
            .file_list
            .model()
            .unwrap()
            .downcast_ref::<gtk4::SingleSelection>()
            .unwrap();

        if let Some(item) = selection.selected_item() {
            let file_obj = item.downcast::<SftpFileObject>().unwrap();
            let file_item = file_obj.file_item();

            let dialog = adw::AlertDialog::builder()
                .heading("Delete File")
                .body(&format!(
                    "Are you sure you want to delete '{}' ?",
                    file_item.name
                ))
                .build();

            dialog.add_response("cancel", "Cancel");
            dialog.add_response("delete", "Delete");
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));

            let path = file_item.path.clone();
            let is_dir = file_item.file_type == SftpFileType::Directory;
            let vm = self.view_model.clone();

            if let Some(ref session_id) = *self.session_id.borrow() {
                let sid = session_id.clone();
                let current = self.current_path.borrow().clone();

                dialog.connect_response(glib::clone!(@weak self as view => move |_, response| {
                    if response == "delete" {
                        std::thread::spawn(glib::clone!(@weak view => move || {
                            let result = if is_dir {
                                vm.sftp_rmdir(&sid, &path)
                            } else {
                                vm.sftp_remove(&sid, &path)
                            };

                            match result {
                                Ok(_) => {
                                    glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                        view.show_message("Deleted successfully");
                                        view.refresh_directory(&current);
                                    }));
                                }
                                Err(e) => {
                                    glib::idle_add_local_once(glib::clone!(@weak view => move || {
                                        view.show_error(&format!("Delete failed: {}", e));
                                    }));
                                }
                            }
                        }));
                    }
                }));
            }

            dialog.present(Some(
                &self.widget.root().and_downcast::<gtk4::Window>().unwrap(),
            ));
        } else {
            self.show_message("No file selected");
        }
    }

    fn show_message(&self, message: &str) {
        self.status_label.set_text(message);
        self.status_label.remove_css_class("error");
    }

    fn show_error(&self, message: &str) {
        self.status_label.set_text(message);
        self.status_label.add_css_class("error");

        if let Some(ref cb) = *self.error_callback.borrow() {
            cb(message);
        }
    }

    pub fn connect_navigate<F: Fn(&str) + 'static>(&self, callback: F) {
        self.navigate_callback.replace(Some(Box::new(callback)));
    }

    pub fn connect_error<F: Fn(&str) + 'static>(&self, callback: F) {
        self.error_callback.replace(Some(Box::new(callback)));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn close(&self) {
        if let Some(ref session_id) = *self.session_id.borrow() {
            let _ = self.view_model.sftp_close(session_id);
        }
    }
}

mod imp {
    use super::*;
    use gtk4::glib;
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
    pub struct SftpFileObject {
        pub item: RefCell<SftpFileItem>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SftpFileObject {
        const NAME: &'static str = "SftpFileObject";
        type Type = super::SftpFileObject;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for SftpFileObject {}

    impl SftpFileObject {
        pub fn new(item: SftpFileItem) -> Self {
            Self {
                item: RefCell::new(item),
            }
        }

        pub fn file_item(&self) -> SftpFileItem {
            self.item.borrow().clone()
        }
    }
}

glib::wrapper! {
    pub struct SftpFileObject(ObjectSubclass<imp::SftpFileObject>);
}

impl SftpFileObject {
    pub fn new(item: SftpFileItem) -> Self {
        let obj: Self = glib::Object::new();
        *obj.imp().item.borrow_mut() = item;
        obj
    }

    pub fn file_item(&self) -> SftpFileItem {
        self.imp().file_item()
    }
}
