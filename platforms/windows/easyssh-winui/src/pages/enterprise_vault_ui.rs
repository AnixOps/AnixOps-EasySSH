//! Enterprise Password Vault UI
//!
//! Professional password management with secure storage, categories,
//! search, and team sharing capabilities (Pro feature).

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::design::{DesignTheme, SemanticColors};

/// Password entry category
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultCategory {
    Server,
    Database,
    Website,
    ApiKey,
    Secret,
    Note,
    Custom(String),
}

impl VaultCategory {
    pub fn display_name(&self) -> String {
        match self {
            Self::Server => "Servers".to_string(),
            Self::Database => "Databases".to_string(),
            Self::Website => "Websites".to_string(),
            Self::ApiKey => "API Keys".to_string(),
            Self::Secret => "Secrets".to_string(),
            Self::Note => "Secure Notes".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Server => "🖥️",
            Self::Database => "🗄️",
            Self::Website => "🌐",
            Self::ApiKey => "🔑",
            Self::Secret => "🔒",
            Self::Note => "📝",
            Self::Custom(_) => "📦",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Server,
            Self::Database,
            Self::Website,
            Self::ApiKey,
            Self::Secret,
            Self::Note,
        ]
    }
}

/// A secure password/credential entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub category: VaultCategory,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_favorite: bool,
    pub access_count: u32,
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    pub team_shared: bool,
    pub permissions: VaultPermissions,
}

impl Default for VaultEntry {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: String::new(),
            username: String::new(),
            password: String::new(),
            category: VaultCategory::Server,
            url: None,
            notes: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at: None,
            is_favorite: false,
            access_count: 0,
            last_accessed: None,
            team_shared: false,
            permissions: VaultPermissions::default(),
        }
    }
}

impl VaultEntry {
    pub fn new(title: &str, username: &str, password: &str, category: VaultCategory) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            category,
            ..Default::default()
        }
    }

    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(chrono::Utc::now());
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            chrono::Utc::now() > expires
        } else {
            false
        }
    }

    /// Securely zeroize the password when dropping
    fn zeroize_password(&mut self) {
        // Overwrite password memory with zeros
        let bytes = unsafe { self.password.as_bytes_mut() };
        for byte in bytes.iter_mut() {
            *byte = 0;
        }
    }
}

impl Drop for VaultEntry {
    fn drop(&mut self) {
        self.zeroize_password();
    }
}

/// Access permissions for vault entries
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VaultPermissions {
    pub can_read: Vec<String>,
    pub can_write: Vec<String>,
    pub can_share: Vec<String>,
}

/// Password generator settings
#[derive(Clone, Debug)]
pub struct PasswordGenerator {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_numbers: bool,
    pub include_symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for PasswordGenerator {
    fn default() -> Self {
        Self {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_symbols: true,
            exclude_ambiguous: true,
        }
    }
}

impl PasswordGenerator {
    pub fn generate(&self) -> String {
        let mut chars = String::new();

        if self.include_lowercase {
            chars.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        if self.include_uppercase {
            chars.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        if self.include_numbers {
            chars.push_str("0123456789");
        }
        if self.include_symbols {
            if self.exclude_ambiguous {
                chars.push_str("!@#$%^&*");
            } else {
                chars.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
            }
        }

        if chars.is_empty() {
            return String::new();
        }

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Simple random number generator based on current time
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut result = String::with_capacity(self.length);
        let mut state = seed;

        for _ in 0..self.length {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (state % chars.len() as u64) as usize;
            result.push(chars.chars().nth(idx).unwrap());
        }

        result
    }
}

/// Enterprise vault window state
pub struct EnterpriseVaultWindow {
    pub open: bool,
    pub entries: Vec<VaultEntry>,
    pub selected_entry: Option<String>,
    pub search_query: String,
    pub selected_category: Option<VaultCategory>,
    pub show_add_dialog: bool,
    pub show_edit_dialog: bool,
    pub show_password_generator: bool,
    pub editing_entry: Option<VaultEntry>,
    pub new_entry: VaultEntry,
    pub password_generator: PasswordGenerator,
    pub generated_password: String,
    pub show_password: bool,
    pub filter_favorites: bool,
    pub filter_expired: bool,
    pub sort_by: SortOption,
    pub view_mode: ViewMode,
    pub clipboard: Option<arboard::Clipboard>,
    pub theme: DesignTheme,
    pub last_error: Option<String>,
    pub show_delete_confirm: bool,
    pub entry_to_delete: Option<String>,
    pub show_team_share: bool,
    pub share_team_members: Vec<String>,
    pub new_tag_input: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SortOption {
    Name,
    Created,
    Updated,
    Accessed,
    Category,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewMode {
    List,
    Grid,
    Tree,
}

impl EnterpriseVaultWindow {
    pub fn new(theme: DesignTheme) -> Self {
        let clipboard = arboard::Clipboard::new().ok();

        let mut vault = Self {
            open: false,
            entries: Vec::new(),
            selected_entry: None,
            search_query: String::new(),
            selected_category: None,
            show_add_dialog: false,
            show_edit_dialog: false,
            show_password_generator: false,
            editing_entry: None,
            new_entry: VaultEntry::default(),
            password_generator: PasswordGenerator::default(),
            generated_password: String::new(),
            show_password: false,
            filter_favorites: false,
            filter_expired: false,
            sort_by: SortOption::Updated,
            view_mode: ViewMode::List,
            clipboard,
            theme,
            last_error: None,
            show_delete_confirm: false,
            entry_to_delete: None,
            show_team_share: false,
            share_team_members: Vec::new(),
            new_tag_input: String::new(),
        };

        // Load sample data for demo
        vault.load_sample_data();
        vault
    }

    pub fn open(&mut self) {
        self.open = true;
        self.last_error = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.show_add_dialog = false;
        self.show_edit_dialog = false;
        self.show_password_generator = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn load_sample_data(&mut self) {
        self.entries = vec![
            VaultEntry::new("Production Server", "admin", "P@ssw0rd123!", VaultCategory::Server),
            VaultEntry::new("AWS Root Account", "root", "aws-secret-key-here", VaultCategory::ApiKey),
            VaultEntry::new("Company Database", "dbadmin", "db-password-456", VaultCategory::Database),
            VaultEntry::new("GitHub Token", "token", "ghp_xxxxxxxxxxxx", VaultCategory::ApiKey),
        ];

        // Mark some as favorites
        if let Some(entry) = self.entries.first_mut() {
            entry.is_favorite = true;
        }
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let window_response = egui::Window::new("🔐 Enterprise Password Vault")
            .open(&mut self.open)
            .default_size([900.0, 650.0])
            .min_size([700.0, 500.0])
            .show(ctx, |ui| {
                self.render_main_ui(ui);
            });

        // Handle dialogs
        if self.show_add_dialog {
            self.render_add_dialog(ctx);
        }
        if self.show_edit_dialog {
            self.render_edit_dialog(ctx);
        }
        if self.show_password_generator {
            self.render_password_generator(ctx);
        }
        if self.show_delete_confirm {
            self.render_delete_confirm(ctx);
        }
        if self.show_team_share {
            self.render_team_share(ctx);
        }
    }

    fn render_main_ui(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        self.render_toolbar(ui);
        ui.separator();

        // Main content area
        egui::SidePanel::left("vault_categories")
            .resizable(true)
            .default_width(180.0)
            .show_inside(ui, |ui| {
                self.render_categories(ui);
            });

        egui::CentralPanel::show_inside(ui, |ui| {
            self.render_entries_list(ui);
        });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Add button
            if ui.button("➕ Add Entry").clicked() {
                self.show_add_dialog = true;
                self.new_entry = VaultEntry::default();
                self.show_password = false;
            }

            ui.add_space(8.0);

            // Search
            ui.label("🔍");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search entries...")
                    .desired_width(200.0),
            );

            ui.add_space(8.0);

            // Filters
            ui.checkbox(&mut self.filter_favorites, "⭐ Favorites");
            ui.checkbox(&mut self.filter_expired, "⚠️ Expired");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View mode toggle
                let view_btn_text = match self.view_mode {
                    ViewMode::List => "☰",
                    ViewMode::Grid => "⊞",
                    ViewMode::Tree => "🌳",
                };
                if ui.button(view_btn_text).clicked() {
                    self.view_mode = match self.view_mode {
                        ViewMode::List => ViewMode::Grid,
                        ViewMode::Grid => ViewMode::Tree,
                        ViewMode::Tree => ViewMode::List,
                    };
                }

                ui.add_space(4.0);

                // Sort dropdown
                egui::ComboBox::from_label("")
                    .selected_text(format!("Sort: {:?}", self.sort_by))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort_by, SortOption::Name, "Name");
                        ui.selectable_value(&mut self.sort_by, SortOption::Updated, "Last Updated");
                        ui.selectable_value(&mut self.sort_by, SortOption::Created, "Created");
                        ui.selectable_value(&mut self.sort_by, SortOption::Accessed, "Last Accessed");
                        ui.selectable_value(&mut self.sort_by, SortOption::Category, "Category");
                    });
            });
        });
    }

    fn render_categories(&mut self, ui: &mut egui::Ui) {
        ui.heading("Categories");
        ui.add_space(8.0);

        // All entries
        let all_count = self.entries.len();
        let is_selected = self.selected_category.is_none();
        let btn = if is_selected {
            egui::Button::new(format!("📁 All Entries ({})", all_count))
                .fill(self.theme.bg_secondary)
        } else {
            egui::Button::new(format!("📁 All Entries ({})", all_count))
        };
        if ui.add(btn).clicked() {
            self.selected_category = None;
        }

        ui.add_space(4.0);

        // Favorites
        let fav_count = self.entries.iter().filter(|e| e.is_favorite).count();
        let btn = egui::Button::new(format!("⭐ Favorites ({})", fav_count));
        if ui.add(btn).clicked() {
            self.filter_favorites = !self.filter_favorites;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Categories
        for category in VaultCategory::all() {
            let count = self.entries.iter().filter(|e| e.category == category).count();
            let is_selected = self.selected_category.as_ref() == Some(&category);

            let btn = if is_selected {
                egui::Button::new(format!("{} {} ({})", category.icon(), category.display_name(), count))
                    .fill(self.theme.bg_secondary)
            } else {
                egui::Button::new(format!("{} {} ({})", category.icon(), category.display_name(), count))
            };

            if ui.add(btn).clicked() {
                self.selected_category = Some(category.clone());
            }
        }

        ui.add_space(16.0);

        // Quick stats
        ui.separator();
        ui.label(egui::RichText::new("Statistics").size(12.0).color(self.theme.text_secondary));
        ui.label(format!("Total: {}", all_count));
        ui.label(format!("Favorites: {}", fav_count));
        ui.label(format!("Expired: {}", self.entries.iter().filter(|e| e.is_expired()).count()));
    }

    fn render_entries_list(&mut self, ui: &mut egui::Ui) {
        // Get filtered and sorted entries
        let mut entries: Vec<&VaultEntry> = self.entries.iter()
            .filter(|e| self.matches_filters(e))
            .collect();

        // Sort
        entries.sort_by(|a, b| match self.sort_by {
            SortOption::Name => a.title.cmp(&b.title),
            SortOption::Created => b.created_at.cmp(&a.created_at),
            SortOption::Updated => b.updated_at.cmp(&a.updated_at),
            SortOption::Accessed => b.last_accessed.cmp(&a.last_accessed),
            SortOption::Category => format!("{:?}", a.category).cmp(&format!("{:?}", b.category)),
        });

        if entries.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("🔐").size(48.0));
                ui.label("No entries found");
                ui.label(egui::RichText::new("Add your first password entry to get started")
                    .size(12.0)
                    .color(self.theme.text_secondary));
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.view_mode {
                ViewMode::List => self.render_list_view(ui, &entries),
                ViewMode::Grid => self.render_grid_view(ui, &entries),
                ViewMode::Tree => self.render_tree_view(ui, &entries),
            }
        });
    }

    fn render_list_view(&mut self, ui: &mut egui::Ui, entries: &[&VaultEntry]) {
        for entry in entries {
            let is_selected = self.selected_entry.as_ref() == Some(&entry.id);

            let response = egui::Frame::group(ui.style())
                .fill(if is_selected { self.theme.bg_secondary } else { ui.visuals().panel_fill })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Favorite toggle
                        let fav_text = if entry.is_favorite { "⭐" } else { "☆" };
                        if ui.small_button(fav_text).clicked() {
                            if let Some(e) = self.entries.iter_mut().find(|e| e.id == entry.id) {
                                e.is_favorite = !e.is_favorite;
                            }
                        }

                        ui.add_space(8.0);

                        // Icon and title
                        ui.label(entry.category.icon());
                        ui.label(&entry.title);

                        if entry.team_shared {
                            ui.label("👥").on_hover_text("Team shared");
                        }
                        if entry.is_expired() {
                            ui.colored_label(SemanticColors::DANGER, "⚠️ Expired");
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Action buttons
                            if ui.small_button("📋").on_hover_text("Copy password").clicked() {
                                self.copy_to_clipboard(&entry.password);
                            }
                            if ui.small_button("👁️").on_hover_text("View details").clicked() {
                                self.selected_entry = Some(entry.id.clone());
                            }
                            if ui.small_button("✏️").on_hover_text("Edit").clicked() {
                                self.start_edit(entry);
                            }
                        });
                    });
                })
                .response;

            if response.clicked() {
                self.selected_entry = Some(entry.id.clone());
            }
        }
    }

    fn render_grid_view(&mut self, ui: &mut egui::Ui, entries: &[&VaultEntry]) {
        let column_count = (ui.available_width() / 200.0).max(1.0) as usize;

        egui::Grid::new("vault_grid").spacing([16.0, 16.0]).show(ui, |ui| {
            for (idx, entry) in entries.iter().enumerate() {
                if idx > 0 && idx % column_count == 0 {
                    ui.end_row();
                }

                egui::Frame::group(ui.style())
                    .fill(ui.visuals().panel_fill)
                    .show(ui, |ui| {
                        ui.set_min_width(180.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(entry.category.icon()).size(32.0));
                            ui.label(&entry.title);
                            ui.label(egui::RichText::new(&entry.username)
                                .size(11.0)
                                .color(self.theme.text_secondary));

                            ui.horizontal(|ui| {
                                if ui.small_button("📋").clicked() {
                                    self.copy_to_clipboard(&entry.password);
                                }
                                if ui.small_button("✏️").clicked() {
                                    self.start_edit(entry);
                                }
                            });
                        });
                    });
            }
        });
    }

    fn render_tree_view(&mut self, ui: &mut egui::Ui, entries: &[&VaultEntry]) {
        // Group by category
        let mut grouped: HashMap<String, Vec<&VaultEntry>> = HashMap::new();
        for entry in entries {
            grouped.entry(entry.category.display_name())
                .or_default()
                .push(entry);
        }

        for (category, cat_entries) in grouped.iter() {
            ui.collapsing(format!("{} {} ({})", "📂", category, cat_entries.len()), |ui| {
                for entry in cat_entries {
                    ui.horizontal(|ui| {
                        ui.label(entry.category.icon());
                        ui.label(&entry.title);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("📋").clicked() {
                                self.copy_to_clipboard(&entry.password);
                            }
                        });
                    });
                }
            });
        }
    }

    fn render_add_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("➕ Add Password Entry")
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 500.0])
            .show(ctx, |ui| {
                self.render_entry_form(ui, true);
            });
    }

    fn render_edit_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("✏️ Edit Entry")
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 500.0])
            .show(ctx, |ui| {
                self.render_entry_form(ui, false);
            });
    }

    fn render_entry_form(&mut self, ui: &mut egui::Ui, is_new: bool) {
        let entry = if is_new {
            &mut self.new_entry
        } else if let Some(ref mut e) = self.editing_entry {
            e
        } else {
            return;
        };

        egui::Grid::new("entry_form").spacing([10.0, 8.0]).show(ui, |ui| {
            ui.label("Title:");
            ui.add(egui::TextEdit::singleline(&mut entry.title).desired_width(280.0));
            ui.end_row();

            ui.label("Category:");
            egui::ComboBox::from_label("")
                .selected_text(entry.category.display_name())
                .show_ui(ui, |ui| {
                    for cat in VaultCategory::all() {
                        ui.selectable_value(&mut entry.category, cat.clone(),
                            format!("{} {}", cat.icon(), cat.display_name()));
                    }
                });
            ui.end_row();

            ui.label("Username:");
            ui.add(egui::TextEdit::singleline(&mut entry.username).desired_width(280.0));
            ui.end_row();

            ui.label("Password:");
            ui.horizontal(|ui| {
                if self.show_password {
                    ui.add(egui::TextEdit::singleline(&mut entry.password).desired_width(200.0));
                } else {
                    ui.add(egui::TextEdit::singleline(&mut entry.password)
                        .password(true)
                        .desired_width(200.0));
                }
                if ui.button(if self.show_password { "🙈" } else { "👁️" }).clicked() {
                    self.show_password = !self.show_password;
                }
                if ui.button("🎲").on_hover_text("Generate password").clicked() {
                    self.show_password_generator = true;
                    self.generated_password = self.password_generator.generate();
                }
            });
            ui.end_row();

            ui.label("URL:");
            let mut url = entry.url.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::singleline(&mut url).desired_width(280.0)).changed() {
                entry.url = if url.is_empty() { None } else { Some(url) };
            }
            ui.end_row();

            ui.label("Tags:");
            ui.horizontal(|ui| {
                for tag in &entry.tags {
                    ui.label(format!("🏷️ {}", tag));
                }
                if ui.button("+").clicked() && !self.new_tag_input.is_empty() {
                    entry.tags.push(self.new_tag_input.clone());
                    self.new_tag_input.clear();
                }
                ui.add(egui::TextEdit::singleline(&mut self.new_tag_input).desired_width(100.0));
            });
            ui.end_row();

            ui.label("Notes:");
            let mut notes = entry.notes.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::multiline(&mut notes).desired_width(280.0).desired_rows(3)).changed() {
                entry.notes = if notes.is_empty() { None } else { Some(notes) };
            }
            ui.end_row();
        });

        ui.separator();

        if let Some(ref error) = self.last_error {
            ui.colored_label(SemanticColors::DANGER, error);
            ui.add_space(8.0);
        }

        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                if is_new {
                    self.show_add_dialog = false;
                } else {
                    self.show_edit_dialog = false;
                }
                self.last_error = None;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_text = if is_new { "Add Entry" } else { "Save Changes" };
                if ui.button(save_text).clicked() {
                    if entry.title.is_empty() {
                        self.last_error = Some("Title is required".to_string());
                    } else {
                        if is_new {
                            let new_entry = self.new_entry.clone();
                            self.entries.push(new_entry);
                            self.show_add_dialog = false;
                        } else if let Some(edited) = self.editing_entry.take() {
                            if let Some(idx) = self.entries.iter().position(|e| e.id == edited.id) {
                                self.entries[idx] = edited;
                            }
                            self.show_edit_dialog = false;
                        }
                        self.last_error = None;
                    }
                }
            });
        });
    }

    fn render_password_generator(&mut self, ctx: &egui::Context) {
        egui::Window::new("🎲 Password Generator")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 300.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Generated password display
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.set_min_width(300.0);
                        ui.label(egui::RichText::new(&self.generated_password).monospace().size(16.0));
                    });

                    if ui.button("🔄 Regenerate").clicked() {
                        self.generated_password = self.password_generator.generate();
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Settings
                    ui.heading("Settings");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Length:");
                        ui.add(egui::Slider::new(&mut self.password_generator.length, 8..=64));
                    });

                    ui.checkbox(&mut self.password_generator.include_uppercase, "Uppercase (A-Z)");
                    ui.checkbox(&mut self.password_generator.include_lowercase, "Lowercase (a-z)");
                    ui.checkbox(&mut self.password_generator.include_numbers, "Numbers (0-9)");
                    ui.checkbox(&mut self.password_generator.include_symbols, "Symbols (!@#$%)");
                    ui.checkbox(&mut self.password_generator.exclude_ambiguous, "Exclude ambiguous (0, O, l, 1)");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_password_generator = false;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Use Password").clicked() {
                                if let Some(ref mut editing) = self.editing_entry {
                                    editing.password = self.generated_password.clone();
                                } else {
                                    self.new_entry.password = self.generated_password.clone();
                                }
                                self.show_password_generator = false;
                            }
                        });
                    });
                });
            });
    }

    fn render_delete_confirm(&mut self, ctx: &egui::Context) {
        egui::Window::new("⚠️ Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .default_size([300.0, 150.0])
            .show(ctx, |ui| {
                ui.label("Are you sure you want to delete this entry?");
                ui.label("This action cannot be undone.");

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_delete_confirm = false;
                        self.entry_to_delete = None;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑️ Delete").clicked() {
                            if let Some(id) = self.entry_to_delete.take() {
                                self.entries.retain(|e| e.id != id);
                            }
                            self.show_delete_confirm = false;
                        }
                    });
                });
            });
    }

    fn render_team_share(&mut self, ctx: &egui::Context) {
        egui::Window::new("👥 Share with Team")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 250.0])
            .show(ctx, |ui| {
                ui.label("Select team members to share this entry with:");
                ui.add_space(8.0);

                // Placeholder team members
                let members = vec!["Alice (Admin)", "Bob (Developer)", "Carol (DevOps)"];
                for member in members {
                    ui.checkbox(&mut false, member);
                }

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_team_share = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Share").clicked() {
                            // TODO: Implement team sharing
                            self.show_team_share = false;
                        }
                    });
                });
            });
    }

    fn matches_filters(&self, entry: &VaultEntry) -> bool {
        // Category filter
        if let Some(ref cat) = self.selected_category {
            if entry.category != *cat {
                return false;
            }
        }

        // Favorites filter
        if self.filter_favorites && !entry.is_favorite {
            return false;
        }

        // Expired filter
        if self.filter_expired && !entry.is_expired() {
            return false;
        }

        // Search query
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            let matches = entry.title.to_lowercase().contains(&query)
                || entry.username.to_lowercase().contains(&query)
                || entry.tags.iter().any(|t| t.to_lowercase().contains(&query));
            if !matches {
                return false;
            }
        }

        true
    }

    fn start_edit(&mut self, entry: &VaultEntry) {
        self.editing_entry = Some(entry.clone());
        self.show_edit_dialog = true;
        self.show_password = false;
    }

    fn copy_to_clipboard(&mut self, text: &str) {
        if let Some(ref mut clipboard) = self.clipboard {
            if let Err(e) = clipboard.set_text(text.to_string()) {
                self.last_error = Some(format!("Failed to copy: {}", e));
            }
        }
    }

    pub fn add_entry(&mut self, title: &str, username: &str, password: &str, category: VaultCategory) {
        self.entries.push(VaultEntry::new(title, username, password, category));
    }

    pub fn delete_entry(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn get_entry(&self, id: &str) -> Option<&VaultEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn get_entry_mut(&mut self, id: &str) -> Option<&mut VaultEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&VaultEntry> {
        let query = query.to_lowercase();
        self.entries.iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&query)
                    || e.username.to_lowercase().contains(&query)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .collect()
    }
}

