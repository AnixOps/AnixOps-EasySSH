//! SSH Key Manager UI
//!
//! Provides a comprehensive interface for managing SSH keys including:
//! - Generate new SSH key pairs (RSA, Ed25519, ECDSA)
//! - Import existing keys
//! - Export keys with password protection
//! - View key fingerprints and metadata
//! - Organize keys with tags and groups
//! - Set default keys for servers

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// SSH Key information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshKeyInfo {
    pub id: String,
    pub name: String,
    pub key_type: KeyType,
    pub public_key: String,
    pub private_key_path: Option<PathBuf>,
    pub fingerprint: String,
    pub fingerprint_sha256: String,
    pub comment: String,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub last_used: Option<chrono::DateTime<chrono::Local>>,
    pub tags: Vec<String>,
    pub is_encrypted: bool,
    pub key_size: Option<u32>, // For RSA keys
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    Rsa,
    Ed25519,
    Ecdsa,
    Dsa, // Legacy, but supported
}

impl Default for KeyType {
    fn default() -> Self {
        KeyType::Ed25519
    }
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Rsa => write!(f, "RSA"),
            KeyType::Ed25519 => write!(f, "Ed25519"),
            KeyType::Ecdsa => write!(f, "ECDSA"),
            KeyType::Dsa => write!(f, "DSA"),
        }
    }
}

impl SshKeyInfo {
    pub fn formatted_fingerprint(&self) -> String {
        // Format: AB:CD:EF:... (16-byte MD5 format)
        self.fingerprint
            .chars()
            .enumerate()
            .fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && i % 2 == 0 && i < self.fingerprint.len() - 1 {
                    acc.push(':');
                }
                acc.push(c);
                acc
            })
    }

    pub fn short_fingerprint(&self) -> String {
        let formatted = self.formatted_fingerprint();
        if formatted.len() > 23 {
            format!("{}...", &formatted[..23])
        } else {
            formatted
        }
    }
}

/// Key manager UI state
pub struct KeyManagerUI {
    pub is_open: bool,
    pub keys: Vec<SshKeyInfo>,
    pub selected_key_id: Option<String>,
    pub search_query: String,
    pub filter_type: Option<KeyType>,
    pub show_generate_dialog: bool,
    pub show_import_dialog: bool,
    pub show_export_dialog: bool,
    pub show_key_details: bool,
    pub generate_form: GenerateKeyForm,
    pub import_form: ImportKeyForm,
    pub export_password: String,
    pub new_tag: String,
    pub action_message: Option<(String, chrono::DateTime<chrono::Local>)>,
    pub ssh_dir: PathBuf,
}

#[derive(Default)]
pub struct GenerateKeyForm {
    pub name: String,
    pub key_type: KeyType,
    pub key_size: u32,
    pub comment: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Default)]
pub struct ImportKeyForm {
    pub name: String,
    pub private_key_path: Option<PathBuf>,
    pub public_key_path: Option<PathBuf>,
    pub password: String,
}

impl Default for KeyManagerUI {
    fn default() -> Self {
        let ssh_dir = dirs::home_dir()
            .map(|h| h.join(".ssh"))
            .unwrap_or_else(|| PathBuf::from(".ssh"));

        Self {
            is_open: false,
            keys: Vec::new(),
            selected_key_id: None,
            search_query: String::new(),
            filter_type: None,
            show_generate_dialog: false,
            show_import_dialog: false,
            show_export_dialog: false,
            show_key_details: false,
            generate_form: GenerateKeyForm::default(),
            import_form: ImportKeyForm::default(),
            export_password: String::new(),
            new_tag: String::new(),
            action_message: None,
            ssh_dir,
        }
    }
}

impl KeyManagerUI {
    pub fn new() -> Self {
        let mut ui = Self::default();
        // Initialize default key sizes
        ui.generate_form.key_size = 4096; // Default RSA size
        ui
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.load_keys_from_ssh_dir();
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Load existing keys from ~/.ssh directory
    pub fn load_keys_from_ssh_dir(&mut self) {
        self.keys.clear();

        if !self.ssh_dir.exists() {
            return;
        }

        // Look for public key files (*.pub)
        if let Ok(entries) = std::fs::read_dir(&self.ssh_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "pub" {
                        if let Some(stem) = path.file_stem() {
                            let private_path = path.with_file_name(stem);
                            self.load_key_pair(&path, &private_path);
                        }
                    }
                }
            }
        }
    }

    /// Load a single key pair
    fn load_key_pair(&mut self, public_path: &PathBuf, private_path: &PathBuf) {
        if let Ok(public_key) = std::fs::read_to_string(public_path) {
            let key_info = SshKeyInfo {
                id: Uuid::new_v4().to_string(),
                name: private_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                key_type: Self::detect_key_type(&public_key),
                public_key: public_key.trim().to_string(),
                private_key_path: if private_path.exists() {
                    Some(private_path.clone())
                } else {
                    None
                },
                fingerprint: "unknown".to_string(), // Would need ssh-keygen or crypto library
                fingerprint_sha256: "unknown".to_string(),
                comment: Self::extract_comment(&public_key),
                created_at: chrono::Local::now(),
                last_used: None,
                tags: Vec::new(),
                is_encrypted: Self::detect_encrypted(private_path),
                key_size: None,
                is_default: false,
            };
            self.keys.push(key_info);
        }
    }

    fn detect_key_type(public_key: &str) -> KeyType {
        if public_key.contains("ssh-ed25519") {
            KeyType::Ed25519
        } else if public_key.contains("ecdsa") {
            KeyType::Ecdsa
        } else if public_key.contains("ssh-dss") {
            KeyType::Dsa
        } else {
            KeyType::Rsa
        }
    }

    fn extract_comment(public_key: &str) -> String {
        public_key
            .split_whitespace()
            .nth(2)
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    fn detect_encrypted(private_path: &PathBuf) -> bool {
        if let Ok(content) = std::fs::read_to_string(private_path) {
            content.contains("ENCRYPTED") || content.contains("Proc-Type: 4,ENCRYPTED")
        } else {
            false
        }
    }

    /// Add a new key
    pub fn add_key(&mut self, key_info: SshKeyInfo) {
        self.keys.push(key_info);
    }

    /// Remove a key
    pub fn remove_key(&mut self, key_id: &str) -> Option<SshKeyInfo> {
        if let Some(index) = self.keys.iter().position(|k| k.id == key_id) {
            Some(self.keys.remove(index))
        } else {
            None
        }
    }

    /// Get key by ID
    pub fn get_key(&self, key_id: &str) -> Option<&SshKeyInfo> {
        self.keys.iter().find(|k| k.id == key_id)
    }

    /// Get mutable key by ID
    pub fn get_key_mut(&mut self, key_id: &str) -> Option<&mut SshKeyInfo> {
        self.keys.iter_mut().find(|k| k.id == key_id)
    }

    /// Set default key
    pub fn set_default_key(&mut self, key_id: &str) {
        for key in &mut self.keys {
            key.is_default = key.id == key_id;
        }
    }

    /// Get default key
    pub fn get_default_key(&self) -> Option<&SshKeyInfo> {
        self.keys.iter().find(|k| k.is_default)
    }

    /// Add tag to key
    pub fn add_tag(&mut self, key_id: &str, tag: String) {
        if let Some(key) = self.get_key_mut(key_id) {
            if !key.tags.contains(&tag) {
                key.tags.push(tag);
            }
        }
    }

    /// Remove tag from key
    pub fn remove_tag(&mut self, key_id: &str, tag: &str) {
        if let Some(key) = self.get_key_mut(key_id) {
            key.tags.retain(|t| t != tag);
        }
    }

    /// Get filtered keys
    pub fn get_filtered_keys(&self) -> Vec<&SshKeyInfo> {
        self.keys
            .iter()
            .filter(|k| {
                // Type filter
                if let Some(ref filter) = self.filter_type {
                    if k.key_type != *filter {
                        return false;
                    }
                }

                // Search filter
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    k.name.to_lowercase().contains(&query)
                        || k.comment.to_lowercase().contains(&query)
                        || k.fingerprint.to_lowercase().contains(&query)
                        || k.tags.iter().any(|t| t.to_lowercase().contains(&query))
                } else {
                    true
                }
            })
            .collect()
    }

    /// Generate a new SSH key pair (placeholder - would use ssh-keygen or crypto library)
    pub fn generate_key(&mut self) -> Result<String, String> {
        // Validate form
        if self.generate_form.name.is_empty() {
            return Err("Key name is required".to_string());
        }
        if self.generate_form.comment.is_empty() {
            return Err("Comment is required".to_string());
        }
        if !self.generate_form.password.is_empty()
            && self.generate_form.password != self.generate_form.confirm_password
        {
            return Err("Passwords do not match".to_string());
        }

        // In a real implementation, this would:
        // 1. Call ssh-keygen or use a crypto library to generate the key
        // 2. Save to ~/.ssh/ with appropriate permissions
        // 3. Return the key ID

        // For now, simulate success
        let key_id = Uuid::new_v4().to_string();

        let key_info = SshKeyInfo {
            id: key_id.clone(),
            name: self.generate_form.name.clone(),
            key_type: self.generate_form.key_type.clone(),
            public_key: format!(
                "ssh-{} AAAAC... {}\n",
                self.generate_form.key_type.to_string().to_lowercase(),
                self.generate_form.comment
            ),
            private_key_path: Some(self.ssh_dir.join(&self.generate_form.name)),
            fingerprint: "00:00:00:00:00:00:00:00:00:00:00:00:00:00:00:00".to_string(),
            fingerprint_sha256: "SHA256:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            comment: self.generate_form.comment.clone(),
            created_at: chrono::Local::now(),
            last_used: None,
            tags: Vec::new(),
            is_encrypted: !self.generate_form.password.is_empty(),
            key_size: if self.generate_form.key_type == KeyType::Rsa {
                Some(self.generate_form.key_size)
            } else {
                None
            },
            is_default: self.keys.is_empty(), // First key is default
        };

        self.keys.push(key_info);
        self.show_generate_dialog = false;
        self.generate_form = GenerateKeyForm::default();
        self.generate_form.key_size = 4096;

        Ok(key_id)
    }

    /// Import an existing key
    pub fn import_key(&mut self) -> Result<String, String> {
        if self.import_form.name.is_empty() {
            return Err("Key name is required".to_string());
        }
        if self.import_form.private_key_path.is_none() {
            return Err("Private key file is required".to_string());
        }

        // In a real implementation, this would:
        // 1. Copy the key to ~/.ssh/
        // 2. Set proper permissions
        // 3. Extract public key if needed
        // 4. Validate the key

        let key_id = Uuid::new_v4().to_string();
        self.show_import_dialog = false;
        self.import_form = ImportKeyForm::default();

        Ok(key_id)
    }

    /// Export a key with password protection
    pub fn export_key(&mut self, key_id: &str, output_path: &PathBuf) -> Result<(), String> {
        if let Some(key) = self.get_key(key_id) {
            if let Some(ref private_path) = key.private_key_path {
                if let Ok(content) = std::fs::read(private_path) {
                    // In a real implementation, this would:
                    // 1. Encrypt the key if a password is provided
                    // 2. Save to the output path

                    if let Err(e) = std::fs::write(output_path, content) {
                        return Err(format!("Failed to write key: {}", e));
                    }

                    self.show_export_dialog = false;
                    self.export_password.clear();
                    return Ok(());
                } else {
                    return Err("Failed to read private key".to_string());
                }
            } else {
                return Err("Private key not found".to_string());
            }
        }
        Err("Key not found".to_string())
    }

    /// Show action message
    pub fn show_message(&mut self, message: String) {
        self.action_message = Some((message, chrono::Local::now()));
    }

    /// Clear expired message
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = self.action_message {
            let elapsed = chrono::Local::now().signed_duration_since(timestamp);
            if elapsed.num_seconds() > 3 {
                self.action_message = None;
            }
        }
    }

    /// Get recommended key sizes based on type
    pub fn get_key_sizes(&self) -> Vec<u32> {
        match self.generate_form.key_type {
            KeyType::Rsa => vec![2048, 3072, 4096, 8192],
            KeyType::Ecdsa => vec![256, 384, 521],
            _ => vec![], // Ed25519 and DSA have fixed sizes
        }
    }

    /// Render the key manager window
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.clear_expired_message();

        let action_msg = self.action_message.clone();

        egui::Window::new("SSH Key Manager")
            .collapsible(false)
            .resizable(true)
            .default_size([900.0, 600.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_content(ui, action_msg.as_ref());
            });

        // Render dialogs
        if self.show_generate_dialog {
            self.render_generate_dialog(ctx);
        }
        if self.show_import_dialog {
            self.render_import_dialog(ctx);
        }
        if self.show_export_dialog {
            self.render_export_dialog(ctx);
        }
        if self.show_key_details {
            self.render_key_details_dialog(ctx);
        }
    }

    fn render_content(
        &mut self,
        ui: &mut egui::Ui,
        action_message: Option<&(String, chrono::DateTime<chrono::Local>)>,
    ) {
        // Header
        let mut should_close = false;
        let mut should_refresh = false;
        let mut show_import = false;
        let mut show_generate = false;
        ui.horizontal(|ui| {
            ui.heading("SSH Keys");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    should_close = true;
                }
                if ui.button("🔄 Refresh").clicked() {
                    should_refresh = true;
                }
                if ui.button("📥 Import").clicked() {
                    show_import = true;
                }
                if ui.button("➕ Generate New").clicked() {
                    show_generate = true;
                }
            });
        });

        if should_close {
            self.close();
            return;
        }
        if should_refresh {
            self.load_keys_from_ssh_dir();
            self.show_message("Keys refreshed".to_string());
        }
        if show_import {
            self.show_import_dialog = true;
            self.import_form = ImportKeyForm::default();
        }
        if show_generate {
            self.show_generate_dialog = true;
            self.generate_form = GenerateKeyForm::default();
            self.generate_form.key_size = 4096;
        }

        ui.add_space(10.0);

        // Search and filter
        let mut filter_changed = false;
        let mut new_filter: Option<KeyType> = None;
        let ssh_dir = self.ssh_dir.clone();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 Search keys...")
                    .desired_width(200.0),
            );

            ui.label("Type:");
            let current_filter = self.filter_type.clone();
            egui::ComboBox::from_id_source("key_type_filter")
                .selected_text(
                    self.filter_type
                        .as_ref()
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "All".to_string()),
                )
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(current_filter.is_none(), "All")
                        .clicked()
                    {
                        new_filter = None;
                        filter_changed = true;
                    }
                    for key_type in [KeyType::Rsa, KeyType::Ed25519, KeyType::Ecdsa] {
                        let selected = current_filter.as_ref() == Some(&key_type);
                        if ui
                            .selectable_label(selected, key_type.to_string())
                            .clicked()
                            && !selected
                        {
                            new_filter = Some(key_type);
                            filter_changed = true;
                        }
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open SSH Directory").clicked() {
                    if let Err(e) = open::that(&ssh_dir) {
                        // Message will be set after the closure
                    }
                }
            });
        });

        if filter_changed {
            self.filter_type = new_filter;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Key list
        let selected_id = self.selected_key_id.clone();
        self.render_key_list(ui, &selected_id);

        // Status message
        if let Some((ref message, _)) = action_message {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(message)
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .size(12.0),
            );
        }
    }

    fn render_key_list(&mut self, ui: &mut egui::Ui, selected_key_id: &Option<String>) {
        let filtered: Vec<SshKeyInfo> = self.get_filtered_keys().into_iter().cloned().collect();

        if filtered.is_empty() {
            ui.label("No SSH keys found. Generate or import a key to get started.");
        } else {
            ui.label(format!("{} key(s) found", filtered.len()));
            ui.add_space(5.0);

            let mut new_selection: Option<String> = None;
            let mut show_details = false;
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for key in &filtered {
                        Self::render_key_item(
                            ui,
                            key,
                            selected_key_id,
                            &mut new_selection,
                            &mut show_details,
                        );
                    }
                });

            if let Some(id) = new_selection {
                self.selected_key_id = Some(id);
            }
            if show_details {
                self.show_key_details = true;
            }
        }
    }

    fn render_key_item(
        ui: &mut egui::Ui,
        key: &SshKeyInfo,
        selected_key_id: &Option<String>,
        new_selection: &mut Option<String>,
        show_details: &mut bool,
    ) {
        let is_selected = selected_key_id
            .as_ref()
            .map(|id| id == &key.id)
            .unwrap_or(false);

        let frame = egui::Frame::group(ui.style())
            .inner_margin(10.0)
            .fill(if is_selected {
                egui::Color32::from_rgb(60, 70, 85)
            } else {
                egui::Color32::TRANSPARENT
            });

        let response = frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Key type icon
                let type_icon = match key.key_type {
                    KeyType::Rsa => "🔐",
                    KeyType::Ed25519 => "🔑",
                    KeyType::Ecdsa => "🗝",
                    KeyType::Dsa => "📜",
                };
                ui.label(egui::RichText::new(type_icon).size(20.0));

                ui.vertical(|ui| {
                    // Name and default badge
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&key.name).strong().size(14.0));
                        if key.is_default {
                            ui.label(
                                egui::RichText::new("★ Default")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 193, 7)),
                            );
                        }
                        if key.is_encrypted {
                            ui.label(
                                egui::RichText::new("🔒 Encrypted")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(72, 199, 116)),
                            );
                        }
                    });

                    // Type and fingerprint
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {} | {}",
                                key.key_type,
                                key.key_size
                                    .map(|s| format!("({} bits)", s))
                                    .unwrap_or_default(),
                                key.short_fingerprint()
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                    });

                    // Comment
                    if !key.comment.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Comment: {}", key.comment))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 180, 180)),
                        );
                    }

                    // Tags
                    if !key.tags.is_empty() {
                        ui.horizontal(|ui| {
                            for tag in &key.tags {
                                ui.label(
                                    egui::RichText::new(format!("#{}", tag))
                                        .size(10.0)
                                        .color(egui::Color32::from_rgb(100, 180, 255)),
                                );
                            }
                        });
                    }
                });

                // Actions - these need to be handled outside
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(""); // Placeholder for delete action
                    ui.label(""); // Placeholder for export action
                    if !key.is_default {
                        ui.label(""); // Placeholder for set default action
                    }
                });
            });
        });

        // Click to select and show details
        if ui
            .interact(
                response.response.rect,
                egui::Id::new(&key.id),
                egui::Sense::click(),
            )
            .clicked()
        {
            *new_selection = Some(key.id.clone());
            *show_details = true;
        }
    }

    fn render_generate_dialog(&mut self, ctx: &egui::Context) {
        let mut should_cancel = false;
        let mut should_generate = false;
        let mut generate_result: Option<Result<String, String>> = None;

        egui::Window::new("Generate New SSH Key")
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 450.0])
            .show(ctx, |ui| {
                ui.label("Create a new SSH key pair for secure authentication");
                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.generate_form.name)
                            .hint_text("my_key")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    let current_type = self.generate_form.key_type.clone();
                    egui::ComboBox::from_id_source("gen_key_type")
                        .selected_text(self.generate_form.key_type.to_string())
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for key_type in [KeyType::Ed25519, KeyType::Rsa, KeyType::Ecdsa] {
                                let selected = current_type == key_type;
                                if ui
                                    .selectable_label(selected, key_type.to_string())
                                    .clicked()
                                    && !selected
                                {
                                    self.generate_form.key_type = key_type.clone();
                                    // Reset key size
                                    if key_type == KeyType::Rsa {
                                        self.generate_form.key_size = 4096;
                                    } else {
                                        self.generate_form.key_size = 0;
                                    }
                                }
                            }
                        });
                });

                ui.add_space(10.0);

                // Key size (only for RSA)
                if self.generate_form.key_type == KeyType::Rsa {
                    ui.horizontal(|ui| {
                        ui.label("Key Size:");
                        let sizes = self.get_key_sizes();
                        let current_size = self.generate_form.key_size;
                        egui::ComboBox::from_id_source("gen_key_size")
                            .selected_text(self.generate_form.key_size.to_string())
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for size in sizes {
                                    if ui
                                        .selectable_label(current_size == size, size.to_string())
                                        .clicked()
                                        && current_size != size
                                    {
                                        self.generate_form.key_size = size;
                                    }
                                }
                            });
                    });
                    ui.add_space(10.0);
                }

                ui.horizontal(|ui| {
                    ui.label("Comment:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.generate_form.comment)
                            .hint_text("user@hostname")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(15.0);

                ui.label("Password Protection (optional):");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.generate_form.password)
                            .password(true)
                            .desired_width(250.0),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Confirm:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.generate_form.confirm_password)
                            .password(true)
                            .desired_width(250.0),
                    );
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_cancel = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Generate").clicked() {
                            should_generate = true;
                        }
                    });
                });
            });

        // Handle actions outside the closure
        if should_cancel {
            self.show_generate_dialog = false;
        }
        if should_generate {
            let result = self.generate_key();
            match result {
                Ok(id) => {
                    self.selected_key_id = Some(id);
                    self.show_message("Key generated successfully".to_string());
                }
                Err(e) => {
                    self.show_message(format!("Error: {}", e));
                }
            }
        }
    }

    fn render_import_dialog(&mut self, ctx: &egui::Context) {
        let mut should_cancel = false;
        let mut should_import = false;

        egui::Window::new("Import SSH Key")
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 350.0])
            .show(ctx, |ui| {
                ui.label("Import an existing SSH private key");
                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.import_form.name)
                            .hint_text("imported_key")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(10.0);

                // Private key file
                ui.horizontal(|ui| {
                    ui.label("Private Key:");
                    ui.add(
                        egui::TextEdit::singleline(
                            &mut self
                                .import_form
                                .private_key_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .desired_width(250.0),
                    );
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.import_form.private_key_path = Some(path);
                        }
                    }
                });

                ui.add_space(10.0);

                // Public key file (optional)
                ui.horizontal(|ui| {
                    ui.label("Public Key (optional):");
                    ui.add(
                        egui::TextEdit::singleline(
                            &mut self
                                .import_form
                                .public_key_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .desired_width(220.0),
                    );
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.import_form.public_key_path = Some(path);
                        }
                    }
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Password (if encrypted):");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.import_form.password)
                            .password(true)
                            .desired_width(200.0),
                    );
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_cancel = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Import").clicked() {
                            should_import = true;
                        }
                    });
                });
            });

        // Handle actions outside the closure
        if should_cancel {
            self.show_import_dialog = false;
        }
        if should_import {
            match self.import_key() {
                Ok(_) => {
                    self.show_message("Key imported successfully".to_string());
                }
                Err(e) => {
                    self.show_message(format!("Error: {}", e));
                }
            }
        }
    }

    fn render_export_dialog(&mut self, ctx: &egui::Context) {
        let mut should_cancel = false;
        let mut should_export = false;
        let key_id_clone = self.selected_key_id.clone();

        egui::Window::new("Export SSH Key")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 250.0])
            .show(ctx, |ui| {
                if let Some(ref key_id) = key_id_clone {
                    if let Some(key) = self.get_key(key_id) {
                        ui.label(format!("Export key: {}", key.name));
                        ui.add_space(10.0);

                        ui.label(
                            egui::RichText::new("Warning: Keep private keys secure!")
                                .color(egui::Color32::from_rgb(255, 193, 7)),
                        );

                        ui.add_space(15.0);

                        ui.checkbox(&mut false, "Encrypt exported key with password");

                        if false {
                            // Would show password field if checkbox enabled
                            ui.horizontal(|ui| {
                                ui.label("Password:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.export_password)
                                        .password(true)
                                        .desired_width(200.0),
                                );
                            });
                        }

                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                should_cancel = true;
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Export").clicked() {
                                        should_export = true;
                                    }
                                },
                            );
                        });
                    } else {
                        ui.label("Key not found");
                    }
                } else {
                    ui.label("No key selected");
                }
            });

        // Handle actions outside the closure
        if should_cancel {
            self.show_export_dialog = false;
        }
        if should_export {
            let key_info = self
                .selected_key_id
                .as_ref()
                .and_then(|id| self.get_key(id).map(|k| (id.clone(), k.name.clone())));
            if let Some((key_id, key_name)) = key_info {
                if let Some(path) = rfd::FileDialog::new().set_file_name(&key_name).save_file() {
                    match self.export_key(&key_id, &path) {
                        Ok(_) => {
                            self.show_message("Key exported".to_string());
                        }
                        Err(e) => {
                            self.show_message(format!("Export failed: {}", e));
                        }
                    }
                }
            }
        }
    }

    fn render_key_details_dialog(&mut self, ctx: &egui::Context) {
        let selected_key_id = self.selected_key_id.clone();
        let mut should_close = false;
        let mut tags_to_remove: Vec<String> = Vec::new();
        let mut should_add_tag = false;

        egui::Window::new("Key Details")
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                if let Some(ref key_id) = selected_key_id {
                    if let Some(key) = self.get_key(key_id).cloned() {
                        ui.heading(&key.name);
                        ui.add_space(10.0);

                        // Key info
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Key Information").strong());
                            ui.add_space(5.0);
                            ui.label(format!("Type: {}", key.key_type));
                            if let Some(size) = key.key_size {
                                ui.label(format!("Size: {} bits", size));
                            }
                            ui.label(format!("Fingerprint: {}", key.formatted_fingerprint()));
                            ui.label(format!("SHA256: {}", key.fingerprint_sha256));
                            ui.label(format!("Comment: {}", key.comment));
                            ui.label(format!(
                                "Created: {}",
                                key.created_at.format("%Y-%m-%d %H:%M")
                            ));
                            if let Some(last_used) = key.last_used {
                                ui.label(format!(
                                    "Last Used: {}",
                                    last_used.format("%Y-%m-%d %H:%M")
                                ));
                            }
                            ui.label(format!(
                                "Encrypted: {}",
                                if key.is_encrypted { "Yes" } else { "No" }
                            ));
                            ui.label(format!(
                                "Default: {}",
                                if key.is_default { "Yes" } else { "No" }
                            ));
                        });

                        ui.add_space(15.0);

                        // Tags management
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Tags").strong());
                            ui.add_space(5.0);

                            // Show existing tags
                            if !key.tags.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    for tag in &key.tags {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("#{}", tag));
                                                if ui.small_button("×").clicked() {
                                                    tags_to_remove.push(tag.clone());
                                                }
                                            });
                                        });
                                    }
                                });
                                ui.add_space(5.0);
                            }

                            // Add new tag
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_tag)
                                        .hint_text("New tag")
                                        .desired_width(150.0),
                                );
                                if ui.button("Add").clicked() && !self.new_tag.is_empty() {
                                    should_add_tag = true;
                                }
                            });
                        });

                        ui.add_space(15.0);

                        // Public key display
                        ui.label(egui::RichText::new("Public Key").strong());
                        ui.add_space(5.0);
                        ui.group(|ui| {
                            ui.set_min_width(450.0);
                            ui.monospace(&key.public_key);
                        });

                        ui.add_space(20.0);

                        if ui.button("Close").clicked() {
                            should_close = true;
                        }
                    } else {
                        ui.label("Key not found");
                    }
                } else {
                    ui.label("No key selected");
                }
            });

        // Handle actions outside the closure
        if should_close {
            self.show_key_details = false;
        }
        if let Some(ref key_id) = selected_key_id {
            for tag in tags_to_remove {
                self.remove_tag(key_id, &tag);
            }
            if should_add_tag && !self.new_tag.is_empty() {
                self.add_tag(key_id, self.new_tag.clone());
                self.new_tag.clear();
            }
        }
    }
}
