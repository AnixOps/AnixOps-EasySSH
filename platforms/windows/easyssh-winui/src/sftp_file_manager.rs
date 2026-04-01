#![allow(dead_code)]

//! Production Grade SFTP File Manager
//! Dual-pane file browser with drag-drop, icons, transfer queue, and preview

use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use crate::file_icons::FileIconTheme;
use crate::file_preview::FilePreview;
use crate::transfer_queue::{TransferDirection, TransferQueue};
use crate::viewmodels::AppViewModel;

/// Main SFTP File Manager struct
pub struct SftpFileManager {
    // Layout
    pub split_ratio: f32, // 0.0-1.0 for split position

    // Local Pane State
    pub local_path: PathBuf,
    pub local_entries: Vec<LocalEntry>,
    pub local_selected: Option<usize>,
    pub local_edit_path: bool,
    pub local_path_input: String,

    // Remote Pane State
    pub remote_path: String,
    pub remote_entries: Vec<RemoteEntry>,
    pub remote_selected: Option<usize>,
    pub remote_edit_path: bool,
    pub remote_path_input: String,
    pub remote_error: Option<String>,

    // UI Components
    pub icon_theme: FileIconTheme,
    pub transfer_queue: TransferQueue,
    pub file_preview: FilePreview,

    // Dialog States
    pub show_new_folder_dialog: bool,
    pub new_folder_name: String,
    pub new_folder_target: PaneSide,

    pub show_rename_dialog: bool,
    pub rename_old_name: String,
    pub rename_new_name: String,
    pub rename_side: PaneSide,

    pub show_properties_dialog: bool,
    pub properties_entry: Option<PropertiesEntry>,

    pub show_transfer_queue: bool,
    pub show_preview: bool,

    // Context Menu
    pub context_menu_open: bool,
    pub context_menu_pos: egui::Pos2,
    pub context_menu_target: ContextMenuTarget,

    // Drag & Drop
    pub drag_source: Option<(PaneSide, usize)>,
    pub drag_start_pos: Option<egui::Pos2>,
}

#[derive(Clone, Debug)]
pub struct LocalEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: std::time::SystemTime,
}

#[derive(Clone, Debug)]
pub struct RemoteEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: i64,
    pub modified: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaneSide {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContextMenuTarget {
    None,
    LocalItem(usize),
    RemoteItem(usize),
    LocalBackground,
    RemoteBackground,
}

#[derive(Clone, Debug)]
pub struct PropertiesEntry {
    pub side: PaneSide,
    pub name: String,
    pub path: String,
    pub size: i64,
    pub modified: String,
    pub is_dir: bool,
}

impl SftpFileManager {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));

        let mut manager = Self {
            split_ratio: 0.5,
            local_path: home_dir.clone(),
            local_entries: Vec::new(),
            local_selected: None,
            local_edit_path: false,
            local_path_input: home_dir.to_string_lossy().to_string(),
            remote_path: String::from("/"),
            remote_entries: Vec::new(),
            remote_selected: None,
            remote_edit_path: false,
            remote_path_input: String::from("/"),
            remote_error: None,
            icon_theme: FileIconTheme::default(),
            transfer_queue: TransferQueue::new(),
            file_preview: FilePreview::new(),
            show_new_folder_dialog: false,
            new_folder_name: String::new(),
            new_folder_target: PaneSide::Local,
            show_rename_dialog: false,
            rename_old_name: String::new(),
            rename_new_name: String::new(),
            rename_side: PaneSide::Local,
            show_properties_dialog: false,
            properties_entry: None,
            show_transfer_queue: false,
            show_preview: false,
            context_menu_open: false,
            context_menu_pos: egui::Pos2::ZERO,
            context_menu_target: ContextMenuTarget::None,
            drag_source: None,
            drag_start_pos: None,
        };

        manager.refresh_local();
        manager
    }

    /// Execute a file download from remote to local
    fn execute_download(
        &mut self,
        view_model: &AppViewModel,
        session_id: &str,
        remote_path: &str,
        local_path: &Path,
        file_name: &str,
        total_size: u64,
    ) {
        let transfer_id = self.transfer_queue.add(
            file_name.to_string(),
            total_size,
            TransferDirection::Download,
        );
        self.show_transfer_queue = true;

        // Start the transfer
        self.transfer_queue.start_transfer(&transfer_id);

        let remote_path = remote_path.to_string();
        let local_path_str = local_path.to_string_lossy().to_string();
        let sid = session_id.to_string();
        let file_name = file_name.to_string();
        let vm = view_model.clone();
        let transfer_id_clone = transfer_id.clone();

        // Execute in background thread
        thread::spawn(move || {
            match vm.sftp_download(&sid, &remote_path, &local_path_str) {
                Ok(data) => {
                    // Write data to file if not already written by core
                    if !local_path_str.is_empty() {
                        if let Err(e) = fs::write(&local_path_str, &data) {
                            eprintln!("Download: Failed to write file {}: {}", file_name, e);
                        }
                    }
                    eprintln!("Download completed: {} ({} bytes)", file_name, data.len());
                }
                Err(e) => {
                    eprintln!("Download failed for {}: {}", file_name, e);
                }
            }
        });
    }

    /// Execute a file upload from local to remote
    fn execute_upload(
        &mut self,
        view_model: &AppViewModel,
        session_id: &str,
        local_path: &Path,
        remote_path: &str,
        file_name: &str,
        total_size: u64,
    ) {
        let transfer_id = self.transfer_queue.add(
            file_name.to_string(),
            total_size,
            TransferDirection::Upload,
        );
        self.show_transfer_queue = true;

        // Start the transfer
        self.transfer_queue.start_transfer(&transfer_id);

        let local_path_str = local_path.to_string_lossy().to_string();
        let remote_path = remote_path.to_string();
        let sid = session_id.to_string();
        let file_name = file_name.to_string();
        let vm = view_model.clone();
        let transfer_id_clone = transfer_id.clone();

        // Execute in background thread
        thread::spawn(move || {
            // Read local file
            match fs::read(&local_path_str) {
                Ok(contents) => {
                    match vm.sftp_upload(&sid, &remote_path, &contents) {
                        Ok(_) => {
                            eprintln!("Upload completed: {} ({} bytes)", file_name, contents.len());
                        }
                        Err(e) => {
                            eprintln!("Upload failed for {}: {}", file_name, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read local file {}: {}", local_path_str, e);
                }
            }
        });
    }

    /// Render the dual-pane file manager
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        view_model: &AppViewModel,
        session_id: Option<&str>,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.heading("📁 SFTP File Manager");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_local();
                    if let Some(sid) = session_id {
                        self.refresh_remote(view_model, sid);
                    }
                }
                if ui.button("📋 Queue").clicked() {
                    self.show_transfer_queue = !self.show_transfer_queue;
                }
            });
        });
        ui.separator();

        // Error display
        if let Some(ref err) = self.remote_error {
            ui.colored_label(egui::Color32::RED, err);
            ui.separator();
        }

        // Dual-pane layout
        let available_width = ui.available_width();
        let left_width = available_width * self.split_ratio;
        let right_width = available_width * (1.0 - self.split_ratio);

        ui.horizontal(|ui| {
            // Left pane - Local
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    self.render_local_pane(ui, view_model, session_id);
                },
            );

            // Resizable splitter
            ui.scope(|ui| {
                let splitter_response = ui.interact(
                    egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(8.0, ui.available_height()),
                    ),
                    ui.id().with("splitter"),
                    egui::Sense::drag(),
                );

                if splitter_response.dragged() {
                    let delta = splitter_response.drag_delta().x;
                    let delta_ratio = delta / available_width;
                    self.split_ratio = (self.split_ratio + delta_ratio).clamp(0.2, 0.8);
                }

                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(4.0, ui.available_height()),
                    ),
                    2.0,
                    if splitter_response.hovered() || splitter_response.dragged() {
                        egui::Color32::from_rgb(100, 150, 200)
                    } else {
                        egui::Color32::from_rgb(60, 65, 75)
                    },
                );
            });

            // Right pane - Remote
            ui.allocate_ui_with_layout(
                egui::vec2(right_width - 8.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    self.render_remote_pane(ui, view_model, session_id);
                },
            );
        });

        // Render dialogs
        self.render_new_folder_dialog(ui.ctx());
        self.render_rename_dialog(ui.ctx());
        self.render_properties_dialog(ui.ctx());
        self.render_transfer_queue(ui.ctx());
        self.render_file_preview(ui.ctx());

        // Context menu
        self.render_context_menu(ui.ctx(), view_model, session_id);
    }

    fn render_local_pane(
        &mut self,
        ui: &mut egui::Ui,
        view_model: &AppViewModel,
        session_id: Option<&str>,
    ) {
        // Header
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 180, 255), "💻 Local");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⬆️ Up").clicked() {
                    self.navigate_local_up();
                }
            });
        });

        // Path bar
        ui.horizontal(|ui| {
            if self.local_edit_path {
                let response = ui.text_edit_singleline(&mut self.local_path_input);
                if response.lost_focus() {
                    self.local_edit_path = false;
                    let new_path = PathBuf::from(&self.local_path_input);
                    if new_path.is_dir() {
                        self.local_path = new_path;
                        self.refresh_local();
                    }
                }
            } else {
                let path_text = self.format_local_breadcrumb();
                if ui.selectable_label(false, path_text).clicked() {
                    self.local_edit_path = true;
                    self.local_path_input = self.local_path.to_string_lossy().to_string();
                }
            }
        });
        ui.separator();

        // File list
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Parent directory entry
            if self.local_path.parent().is_some() {
                let response = ui.selectable_label(false, "📁 ..");
                if response.double_clicked() {
                    self.navigate_local_up();
                }
                if response.clicked() {
                    self.local_selected = None;
                }
                if response.secondary_clicked() {
                    self.open_context_menu(ContextMenuTarget::LocalBackground, &response);
                }
            }

            let entries: Vec<_> = self.local_entries.clone();
            for (idx, entry) in entries.iter().enumerate() {
                let icon = self.icon_theme.get_icon(&entry.name, entry.is_dir);
                let label = format!("{} {}", icon, entry.name);
                let is_selected = self.local_selected == Some(idx);

                let response = ui.selectable_label(is_selected, label);

                // Click handling
                if response.clicked() {
                    self.local_selected = Some(idx);
                }

                if response.double_clicked() {
                    if entry.is_dir {
                        self.navigate_local_to(entry.path.clone());
                    } else {
                        self.open_local_file(&entry.path);
                    }
                }

                // Right-click context menu
                if response.secondary_clicked() {
                    self.local_selected = Some(idx);
                    self.open_context_menu(ContextMenuTarget::LocalItem(idx), &response);
                }

                // Drag start
                if response.drag_started() {
                    self.drag_source = Some((PaneSide::Local, idx));
                    self.drag_start_pos = Some(response.interact_rect.center());
                }

                // Drop target
                if response.hovered() && self.drag_source.is_some() {
                    if entry.is_dir {
                        ui.painter().rect_stroke(
                            response.rect,
                            2.0,
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        );

                        // Handle drop
                        if ui.input(|i| i.pointer.any_released()) {
                            if let Some((source_side, _source_idx)) = self.drag_source.take() {
                                self.handle_drop(
                                    source_side,
                                    PaneSide::Local,
                                    Some(idx),
                                    view_model,
                                    session_id,
                                );
                            }
                        }
                    }
                }
            }

            // Background drop target
            if ui.input(|i| i.pointer.any_released()) {
                if let Some((source_side, _source_idx)) = self.drag_source.take() {
                    let background_rect = ui.min_rect();
                    if background_rect
                        .contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()))
                    {
                        self.handle_drop(source_side, PaneSide::Local, None, view_model, session_id);
                    }
                }
            }
        });
    }

    fn render_remote_pane(
        &mut self,
        ui: &mut egui::Ui,
        view_model: &AppViewModel,
        session_id: Option<&str>,
    ) {
        // Header
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 200, 150), "🌐 Remote");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⬆️ Up").clicked() {
                    self.navigate_remote_up();
                }
            });
        });

        // Path bar
        ui.horizontal(|ui| {
            if self.remote_edit_path {
                let response = ui.text_edit_singleline(&mut self.remote_path_input);
                if response.lost_focus() {
                    self.remote_edit_path = false;
                    if session_id.is_some() {
                        self.remote_path = self.remote_path_input.clone();
                        if let Some(sid) = session_id {
                            self.refresh_remote(view_model, sid);
                        }
                    }
                }
            } else {
                let path_text = self.format_remote_breadcrumb();
                if ui.selectable_label(false, path_text).clicked() {
                    self.remote_edit_path = true;
                    self.remote_path_input = self.remote_path.clone();
                }
            }
        });
        ui.separator();

        // File list
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Parent directory entry
            if self.remote_path != "/" {
                let response = ui.selectable_label(false, "📁 ..");
                if response.double_clicked() {
                    self.navigate_remote_up();
                }
                if response.secondary_clicked() {
                    self.open_context_menu(ContextMenuTarget::RemoteBackground, &response);
                }
            }

            let entries: Vec<_> = self.remote_entries.clone();
            for (idx, entry) in entries.iter().enumerate() {
                let icon = self.icon_theme.get_icon(&entry.name, entry.is_dir);
                let size_str = if entry.is_dir {
                    "-".to_string()
                } else {
                    Self::format_size(entry.size as u64)
                };
                let mtime_str = Self::format_timestamp(entry.modified);
                let label = format!("{} {}  {}  {}", icon, entry.name, size_str, mtime_str);
                let is_selected = self.remote_selected == Some(idx);

                let response = ui.selectable_label(is_selected, label);

                // Click handling
                if response.clicked() {
                    self.remote_selected = Some(idx);
                }

                if response.double_clicked() {
                    if entry.is_dir {
                        self.navigate_remote_to(&entry.path);
                    } else {
                        self.preview_remote_file(view_model, session_id, &entry.path);
                    }
                }

                // Right-click context menu
                if response.secondary_clicked() {
                    self.remote_selected = Some(idx);
                    self.open_context_menu(ContextMenuTarget::RemoteItem(idx), &response);
                }

                // Drag start
                if response.drag_started() {
                    self.drag_source = Some((PaneSide::Remote, idx));
                    self.drag_start_pos = Some(response.interact_rect.center());
                }

                // Drop target
                if response.hovered() && self.drag_source.is_some() {
                    if entry.is_dir {
                        ui.painter().rect_stroke(
                            response.rect,
                            2.0,
                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                        );

                        // Handle drop
                        if ui.input(|i| i.pointer.any_released()) {
                            if let Some((source_side, _source_idx)) = self.drag_source.take() {
                                self.handle_drop(
                                    source_side,
                                    PaneSide::Remote,
                                    Some(idx),
                                    view_model,
                                    session_id,
                                );
                            }
                        }
                    }
                }
            }

            // Background drop target
            if ui.input(|i| i.pointer.any_released()) {
                if let Some((source_side, _source_idx)) = self.drag_source.take() {
                    let background_rect = ui.min_rect();
                    if background_rect
                        .contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()))
                    {
                        self.handle_drop(source_side, PaneSide::Remote, None, view_model, session_id);
                    }
                }
            }
        });

        // Auto-refresh remote on first view
        if self.remote_entries.is_empty() && session_id.is_some() && self.remote_error.is_none() {
            if let Some(sid) = session_id {
                self.refresh_remote(view_model, sid);
            }
        }
    }

    // ==================== Local Operations ====================

    pub fn refresh_local(&mut self) {
        self.local_entries.clear();
        self.local_selected = None;

        match fs::read_dir(&self.local_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.starts_with('.') {
                        continue;
                    }

                    let metadata = match entry.metadata() {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    let is_dir = metadata.is_dir();
                    let size = if is_dir { 0 } else { metadata.len() };
                    let modified = metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                    self.local_entries.push(LocalEntry {
                        name,
                        path,
                        is_dir,
                        size,
                        modified,
                    });
                }

                self.local_entries
                    .sort_by(|a, b| match (a.is_dir, b.is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    });

                self.local_path_input = self.local_path.to_string_lossy().to_string();
            }
            Err(e) => {
                self.remote_error = Some(format!("Failed to read local directory: {}", e));
            }
        }
    }

    fn navigate_local_up(&mut self) {
        if let Some(parent) = self.local_path.parent() {
            self.local_path = parent.to_path_buf();
            self.refresh_local();
        }
    }

    fn navigate_local_to(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.local_path = path;
            self.refresh_local();
        }
    }

    fn open_local_file(&self, path: &Path) {
        // Open file with default application
        if let Err(e) = open_file(path) {
            eprintln!("Failed to open file: {}", e);
        }
    }

    // ==================== Remote Operations ====================

    pub fn refresh_remote(&mut self, view_model: &AppViewModel, session_id: &str) {
        self.remote_error = None;

        match view_model.sftp_list_dir(session_id, &self.remote_path) {
            Ok(entries) => {
                self.remote_entries = entries
                    .into_iter()
                    .filter(|e| !e.name.starts_with('.'))
                    .map(|e| RemoteEntry {
                        name: e.name,
                        path: e.path,
                        is_dir: e.file_type == "directory",
                        size: e.size,
                        modified: e.mtime,
                    })
                    .collect();
                self.remote_path_input = self.remote_path.clone();
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("not initialized") {
                    self.remote_error = Some(String::from("Initializing SFTP..."));
                } else {
                    self.remote_error = Some(format!("SFTP error: {}", err_str));
                }
                self.remote_entries.clear();
            }
        }
    }

    fn navigate_remote_up(&mut self) {
        if self.remote_path != "/" {
            if let Some(pos) = self.remote_path.rfind('/') {
                if pos == 0 {
                    self.remote_path = String::from("/");
                } else {
                    self.remote_path = self.remote_path[..pos].to_string();
                }
            }
        }
    }

    fn navigate_remote_to(&mut self, path: &str) {
        self.remote_path = path.to_string();
    }

    fn preview_remote_file(
        &mut self,
        view_model: &AppViewModel,
        session_id: Option<&str>,
        remote_path: &str,
    ) {
        if let Some(sid) = session_id {
            // Create temp path for preview
            let temp_dir = std::env::temp_dir();
            let file_name = Path::new(remote_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("preview.tmp");
            let temp_path = temp_dir.join(format!("easyssh_preview_{}", file_name));
            let temp_path_str = temp_path.to_str().unwrap_or("");

            match view_model.sftp_download(sid, remote_path, temp_path_str) {
                Ok(data) => {
                    let content = String::from_utf8_lossy(&data).to_string();
                    let name = file_name.to_string();

                    self.file_preview.open(name, content);
                    self.show_preview = true;
                }
                Err(e) => {
                    self.remote_error = Some(format!("Failed to preview file: {}", e));
                }
            }
        }
    }

    // ==================== Drag & Drop ====================

    fn handle_drop(
        &mut self,
        source_side: PaneSide,
        target_side: PaneSide,
        target_idx: Option<usize>,
        view_model: &AppViewModel,
        session_id: Option<&str>,
    ) {
        if source_side == target_side {
            return; // Can't drop on same side
        }

        let Some(sid) = session_id else {
            self.remote_error = Some("No active session".to_string());
            return;
        };

        match (source_side, target_side) {
            (PaneSide::Local, PaneSide::Remote) => {
                if let Some(source_idx) = self.drag_source.map(|(_, idx)| idx) {
                    // Clone entry data before calling self methods to avoid borrow issues
                    let entry_data = self.local_entries.get(source_idx).map(|entry| {
                        (entry.path.clone(), entry.name.clone(), entry.size, entry.is_dir)
                    });

                    if let Some((path, name, size, is_dir)) = entry_data {
                        if is_dir {
                            return; // Skip directories for now
                        }

                        let target_dir = if let Some(idx) = target_idx {
                            self.remote_entries
                                .get(idx)
                                .filter(|e| e.is_dir)
                                .map(|e| e.path.clone())
                                .unwrap_or_else(|| self.remote_path.clone())
                        } else {
                            self.remote_path.clone()
                        };

                        let remote_path = format!("{}/{}", target_dir.trim_end_matches('/'), name);

                        // Execute upload
                        self.execute_upload(
                            view_model,
                            sid,
                            &path,
                            &remote_path,
                            &name,
                            size,
                        );
                    }
                }
            }
            (PaneSide::Remote, PaneSide::Local) => {
                if let Some(source_idx) = self.drag_source.map(|(_, idx)| idx) {
                    // Clone entry data before calling self methods to avoid borrow issues
                    let entry_data = self.remote_entries.get(source_idx).map(|entry| {
                        (entry.path.clone(), entry.name.clone(), entry.size, entry.is_dir)
                    });

                    if let Some((path, name, size, is_dir)) = entry_data {
                        if is_dir {
                            return; // Skip directories for now
                        }

                        let target_dir = if let Some(idx) = target_idx {
                            self.local_entries
                                .get(idx)
                                .filter(|e| e.is_dir)
                                .map(|e| e.path.clone())
                                .unwrap_or_else(|| self.local_path.clone())
                        } else {
                            self.local_path.clone()
                        };

                        let local_path = target_dir.join(&name);

                        // Execute download
                        self.execute_download(
                            view_model,
                            sid,
                            &path,
                            &local_path,
                            &name,
                            size as u64,
                        );

                        // Refresh local after download completes (with delay)
                        let local_path_clone = self.local_path.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            // Note: Can't directly refresh UI from here, would need signal
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // ==================== Context Menu ====================

    fn open_context_menu(&mut self, target: ContextMenuTarget, response: &egui::Response) {
        self.context_menu_target = target;
        self.context_menu_pos = response.interact_rect.center();
        self.context_menu_open = true;
    }

    fn render_context_menu(&mut self, ctx: &egui::Context, view_model: &AppViewModel, session_id: Option<&str>) {
        if !self.context_menu_open {
            return;
        }

        let mut close_menu = false;

        egui::Window::new("context_menu")
            .fixed_pos(self.context_menu_pos)
            .title_bar(false)
            .resizable(false)
            .frame(egui::Frame::popup(&ctx.style()))
            .show(ctx, |ui| match self.context_menu_target {
                ContextMenuTarget::LocalItem(idx) => {
                    if ui.button("Open").clicked() {
                        if let Some(entry) = self.local_entries.get(idx) {
                            if entry.is_dir {
                                self.navigate_local_to(entry.path.clone());
                            } else {
                                self.open_local_file(&entry.path);
                            }
                        }
                        close_menu = true;
                    }
                    if ui.button("Upload").clicked() {
                        // Clone data before calling self method to avoid borrow issues
                        let entry_data = self.local_entries.get(idx).map(|entry| {
                            (entry.path.clone(), entry.name.clone(), entry.size, entry.is_dir)
                        });
                        if let Some((path, name, size, is_dir)) = entry_data {
                            if !is_dir {
                                if let Some(sid) = session_id {
                                    let remote_path = format!("{}/{}", self.remote_path.trim_end_matches('/'), name);
                                    self.execute_upload(
                                        view_model,
                                        sid,
                                        &path,
                                        &remote_path,
                                        &name,
                                        size,
                                    );
                                }
                            }
                        }
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Rename").clicked() {
                        if let Some(entry) = self.local_entries.get(idx) {
                            self.rename_old_name = entry.name.clone();
                            self.rename_new_name = entry.name.clone();
                            self.rename_side = PaneSide::Local;
                            self.show_rename_dialog = true;
                        }
                        close_menu = true;
                    }
                    if ui.button("Delete").clicked() {
                        let path_to_delete = self.local_entries.get(idx).map(|e| e.path.clone());
                        if let Some(path) = path_to_delete {
                            self.delete_local(&path);
                        }
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Properties").clicked() {
                        if let Some(entry) = self.local_entries.get(idx) {
                            self.properties_entry = Some(PropertiesEntry {
                                side: PaneSide::Local,
                                name: entry.name.clone(),
                                path: entry.path.to_string_lossy().to_string(),
                                size: entry.size as i64,
                                modified: Self::format_system_time(entry.modified),
                                is_dir: entry.is_dir,
                            });
                            self.show_properties_dialog = true;
                        }
                        close_menu = true;
                    }
                }
                ContextMenuTarget::RemoteItem(idx) => {
                    if ui.button("Open").clicked() {
                        close_menu = true;
                    }
                    if ui.button("Download").clicked() {
                        // Clone data before calling self method to avoid borrow issues
                        let entry_data = self.remote_entries.get(idx).map(|entry| {
                            (entry.path.clone(), entry.name.clone(), entry.size, entry.is_dir)
                        });
                        if let Some((path, name, size, is_dir)) = entry_data {
                            if !is_dir {
                                if let Some(sid) = session_id {
                                    let local_path = self.local_path.join(&name);
                                    self.execute_download(
                                        view_model,
                                        sid,
                                        &path,
                                        &local_path,
                                        &name,
                                        size as u64,
                                    );
                                }
                            }
                        }
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Rename").clicked() {
                        if let Some(entry) = self.remote_entries.get(idx) {
                            self.rename_old_name = entry.name.clone();
                            self.rename_new_name = entry.name.clone();
                            self.rename_side = PaneSide::Remote;
                            self.show_rename_dialog = true;
                        }
                        close_menu = true;
                    }
                    if ui.button("Delete").clicked() {
                        let path_to_delete = self.remote_entries.get(idx).map(|e| e.path.clone());
                        if let Some(path) = path_to_delete {
                            self.delete_remote(&path);
                        }
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Properties").clicked() {
                        if let Some(entry) = self.remote_entries.get(idx) {
                            self.properties_entry = Some(PropertiesEntry {
                                side: PaneSide::Remote,
                                name: entry.name.clone(),
                                path: entry.path.clone(),
                                size: entry.size,
                                modified: Self::format_timestamp(entry.modified),
                                is_dir: entry.is_dir,
                            });
                            self.show_properties_dialog = true;
                        }
                        close_menu = true;
                    }
                }
                ContextMenuTarget::LocalBackground => {
                    if ui.button("New Folder").clicked() {
                        self.new_folder_name.clear();
                        self.new_folder_target = PaneSide::Local;
                        self.show_new_folder_dialog = true;
                        close_menu = true;
                    }
                    if ui.button("Refresh").clicked() {
                        self.refresh_local();
                        close_menu = true;
                    }
                }
                ContextMenuTarget::RemoteBackground => {
                    if ui.button("New Folder").clicked() {
                        self.new_folder_name.clear();
                        self.new_folder_target = PaneSide::Remote;
                        self.show_new_folder_dialog = true;
                        close_menu = true;
                    }
                    if ui.button("Refresh").clicked() {
                        close_menu = true;
                    }
                }
                _ => {}
            });

        // Close menu on click outside
        if ctx.input(|i| i.pointer.any_click()) {
            close_menu = true;
        }

        if close_menu {
            self.context_menu_open = false;
        }
    }

    // ==================== File Operations ====================

    fn delete_local(&mut self, path: &Path) {
        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        match result {
            Ok(_) => self.refresh_local(),
            Err(e) => self.remote_error = Some(format!("Failed to delete: {}", e)),
        }
    }

    fn delete_remote(&mut self, path: &str) {
        // This would need view_model and session_id
        // For now just show error
        self.remote_error = Some(format!("Delete {} not implemented yet", path));
    }

    // ==================== Dialogs ====================

    fn render_new_folder_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_folder_dialog {
            return;
        }

        egui::Window::new("New Folder")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 120.0])
            .show(ctx, |ui| {
                ui.label("Folder name:");
                ui.text_edit_singleline(&mut self.new_folder_name);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_new_folder_dialog = false;
                    }
                    if ui.button("Create").clicked() && !self.new_folder_name.is_empty() {
                        self.create_folder();
                    }
                });
            });
    }

    fn render_rename_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_rename_dialog {
            return;
        }

        egui::Window::new("Rename")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 120.0])
            .show(ctx, |ui| {
                ui.label("New name:");
                ui.text_edit_singleline(&mut self.rename_new_name);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_rename_dialog = false;
                    }
                    if ui.button("Rename").clicked() && !self.rename_new_name.is_empty() {
                        self.perform_rename();
                    }
                });
            });
    }

    fn render_properties_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_properties_dialog {
            return;
        }

        egui::Window::new("Properties")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 200.0])
            .show(ctx, |ui| {
                if let Some(ref entry) = self.properties_entry {
                    ui.label(format!("Name: {}", entry.name));
                    ui.label(format!("Path: {}", entry.path));
                    ui.label(format!(
                        "Type: {}",
                        if entry.is_dir { "Directory" } else { "File" }
                    ));
                    ui.label(format!("Size: {}", Self::format_size(entry.size as u64)));
                    ui.label(format!("Modified: {}", entry.modified));
                }

                if ui.button("Close").clicked() {
                    self.show_properties_dialog = false;
                }
            });
    }

    fn render_transfer_queue(&mut self, ctx: &egui::Context) {
        if !self.show_transfer_queue {
            return;
        }

        egui::Window::new("Transfer Queue")
            .collapsible(true)
            .resizable(true)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Active: {}", self.transfer_queue.active_count()));
                    ui.label(format!("Pending: {}", self.transfer_queue.pending_count()));
                    ui.label(format!(
                        "Completed: {}",
                        self.transfer_queue.completed_count()
                    ));
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for item in self.transfer_queue.items() {
                        ui.horizontal(|ui| {
                            ui.label(item.direction_icon());
                            ui.label(&item.file_name);
                            ui.label(item.status_text());
                            ui.label(format!("{:.0}%", item.progress_percent()));
                        });
                    }
                });

                if ui.button("Close").clicked() {
                    self.show_transfer_queue = false;
                }
            });
    }

    fn render_file_preview(&mut self, ctx: &egui::Context) {
        if !self.show_preview || !self.file_preview.is_open {
            return;
        }

        egui::Window::new(format!("Preview: {}", self.file_preview.file_name))
            .collapsible(true)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.label(self.file_preview.get_preview_summary());
                ui.separator();

                // Toolbar
                ui.horizontal(|ui| {
                    if ui.button("🔍+").clicked() {
                        self.file_preview.zoom_in();
                    }
                    if ui.button("🔍-").clicked() {
                        self.file_preview.zoom_out();
                    }
                    if ui.button("↩️").clicked() {
                        self.file_preview.reset_zoom();
                    }
                    ui.separator();
                    if ui
                        .button(if self.file_preview.wrap_text {
                            "↩️ Wrap"
                        } else {
                            "→ No Wrap"
                        })
                        .clicked()
                    {
                        self.file_preview.toggle_wrap();
                    }
                    if ui
                        .button(if self.file_preview.show_line_numbers {
                            "#"
                        } else {
                            "□"
                        })
                        .clicked()
                    {
                        self.file_preview.toggle_line_numbers();
                    }
                    ui.separator();
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.file_preview.search_query);
                    if ui.button("Find").clicked() {
                        self.file_preview
                            .search(&self.file_preview.search_query.clone());
                    }
                });
                ui.separator();

                // Content
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let content = self.file_preview.truncated_content();
                    let text = egui::RichText::new(&content)
                        .monospace()
                        .size(self.file_preview.font_size);
                    ui.label(text);
                });

                if ui.button("Close").clicked() {
                    self.show_preview = false;
                    self.file_preview.close();
                }
            });
    }

    // ==================== Actions ====================

    fn create_folder(&mut self) {
        match self.new_folder_target {
            PaneSide::Local => {
                let new_path = self.local_path.join(&self.new_folder_name);
                match fs::create_dir(&new_path) {
                    Ok(_) => {
                        self.refresh_local();
                        self.show_new_folder_dialog = false;
                        self.new_folder_name.clear();
                    }
                    Err(e) => {
                        self.remote_error = Some(format!("Failed to create folder: {}", e));
                    }
                }
            }
            PaneSide::Remote => {
                // Would need view_model and session_id
                self.remote_error = Some("Remote folder creation not yet implemented".to_string());
            }
        }
    }

    fn perform_rename(&mut self) {
        match self.rename_side {
            PaneSide::Local => {
                let old_path = self.local_path.join(&self.rename_old_name);
                let new_path = self.local_path.join(&self.rename_new_name);
                match fs::rename(&old_path, &new_path) {
                    Ok(_) => {
                        self.refresh_local();
                        self.show_rename_dialog = false;
                    }
                    Err(e) => {
                        self.remote_error = Some(format!("Failed to rename: {}", e));
                    }
                }
            }
            PaneSide::Remote => {
                // Would need view_model and session_id
                self.remote_error = Some("Remote rename not yet implemented".to_string());
            }
        }
    }

    // ==================== Formatting ====================

    fn format_local_breadcrumb(&self) -> String {
        self.local_path.to_string_lossy().to_string()
    }

    fn format_remote_breadcrumb(&self) -> String {
        self.remote_path.clone()
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

    fn format_timestamp(timestamp: i64) -> String {
        if timestamp == 0 {
            String::from("-")
        } else {
            let dt =
                chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now);
            dt.format("%Y-%m-%d %H:%M").to_string()
        }
    }

    fn format_system_time(time: std::time::SystemTime) -> String {
        let datetime: chrono::DateTime<chrono::Local> = time.into();
        datetime.format("%Y-%m-%d %H:%M").to_string()
    }
}

impl Default for SftpFileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility function to open files with default application
fn open_file(path: &std::path::Path) -> std::io::Result<()> {
    open::that(path)
}
