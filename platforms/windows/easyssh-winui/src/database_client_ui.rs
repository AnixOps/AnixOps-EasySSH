#![allow(dead_code)]

//! Database Client UI Integration for EasySSH
//!
//! This module provides UI rendering functions for the database client system,
//! supporting multiple database types with SQL editor and connection management.

use crate::design::{AccessibilitySettings, DesignTheme};
use std::collections::HashMap;

/// Database client panel UI manager
pub struct DatabaseClientPanel {
    pub show_panel: bool,
    pub connections: Vec<DatabaseConnectionUI>,
    pub active_connection: Option<String>,
    pub sql_editor: SqlEditor,
    pub query_results: Vec<QueryResultUI>,
    pub show_add_connection_dialog: bool,
    pub show_query_history: bool,
    pub new_connection_form: NewConnectionForm,
    pub selected_db_type: DatabaseTypeUI,
    pub action_message: Option<(String, std::time::Instant)>,
}

impl DatabaseClientPanel {
    pub fn new() -> Self {
        Self {
            show_panel: false,
            connections: Vec::new(),
            active_connection: None,
            sql_editor: SqlEditor::new(),
            query_results: Vec::new(),
            show_add_connection_dialog: false,
            show_query_history: false,
            new_connection_form: NewConnectionForm::default(),
            selected_db_type: DatabaseTypeUI::MySQL,
            action_message: None,
        }
    }

    pub fn add_connection(&mut self, name: String, db_type: DatabaseTypeUI, host: String, port: u16, database: String, username: String) {
        let id = format!("db_conn_{}", uuid::Uuid::new_v4());
        let conn = DatabaseConnectionUI {
            id: id.clone(),
            name,
            db_type,
            host,
            port,
            database,
            username,
            is_connected: false,
            last_error: None,
            created_at: std::time::Instant::now(),
        };
        self.connections.push(conn);
        self.active_connection = Some(id);
    }

    pub fn disconnect_all(&mut self) {
        for conn in &mut self.connections {
            conn.is_connected = false;
        }
        self.active_connection = None;
    }

    pub fn get_active_connection(&self) -> Option<&DatabaseConnectionUI> {
        self.active_connection.as_ref().and_then(|id| {
            self.connections.iter().find(|c| &c.id == id)
        })
    }

    pub fn get_active_connection_mut(&mut self) -> Option<&mut DatabaseConnectionUI> {
        self.active_connection.as_ref().and_then(|id| {
            self.connections.iter_mut().find(|c| &c.id == id)
        })
    }

    pub fn execute_query(&mut self, query: String) {
        // Mock execution - would connect to real database in production
        let result = QueryResultUI {
            query: query.clone(),
            columns: vec!["Column 1".to_string(), "Column 2".to_string(), "Column 3".to_string()],
            rows: vec![
                vec!["Value 1A".to_string(), "Value 1B".to_string(), "Value 1C".to_string()],
                vec!["Value 2A".to_string(), "Value 2B".to_string(), "Value 2C".to_string()],
                vec!["Value 3A".to_string(), "Value 3B".to_string(), "Value 3C".to_string()],
            ],
            execution_time_ms: 42,
            row_count: 3,
            error: None,
        };
        self.query_results.push(result);
        self.sql_editor.add_to_history(query);
    }

    pub fn show_action_message(&mut self, message: String) {
        self.action_message = Some((message, std::time::Instant::now()));
    }
}

impl Default for DatabaseClientPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DatabaseTypeUI {
    MySQL,
    PostgreSQL,
    MongoDB,
    Redis,
    SQLite,
}

impl DatabaseTypeUI {
    pub fn display_name(&self) -> &'static str {
        match self {
            DatabaseTypeUI::MySQL => "MySQL",
            DatabaseTypeUI::PostgreSQL => "PostgreSQL",
            DatabaseTypeUI::MongoDB => "MongoDB",
            DatabaseTypeUI::Redis => "Redis",
            DatabaseTypeUI::SQLite => "SQLite",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DatabaseTypeUI::MySQL => "🐬",
            DatabaseTypeUI::PostgreSQL => "🐘",
            DatabaseTypeUI::MongoDB => "🍃",
            DatabaseTypeUI::Redis => "🔴",
            DatabaseTypeUI::SQLite => "🪶",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DatabaseTypeUI::MySQL => 3306,
            DatabaseTypeUI::PostgreSQL => 5432,
            DatabaseTypeUI::MongoDB => 27017,
            DatabaseTypeUI::Redis => 6379,
            DatabaseTypeUI::SQLite => 0,
        }
    }

    pub fn supports_sql(&self) -> bool {
        matches!(self, DatabaseTypeUI::MySQL | DatabaseTypeUI::PostgreSQL | DatabaseTypeUI::SQLite)
    }

    pub fn all_types() -> Vec<DatabaseTypeUI> {
        vec![
            DatabaseTypeUI::MySQL,
            DatabaseTypeUI::PostgreSQL,
            DatabaseTypeUI::MongoDB,
            DatabaseTypeUI::Redis,
            DatabaseTypeUI::SQLite,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseConnectionUI {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseTypeUI,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub is_connected: bool,
    pub last_error: Option<String>,
    pub created_at: std::time::Instant,
}

#[derive(Clone, Debug)]
pub struct QueryResultUI {
    pub query: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub execution_time_ms: u64,
    pub row_count: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct NewConnectionForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub use_ssh_tunnel: bool,
    pub ssh_server_id: Option<String>,
    pub ssl_mode: SslModeUI,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SslModeUI {
    #[default]
    Preferred,
    Disabled,
    Required,
    VerifyCa,
    VerifyIdentity,
}

impl SslModeUI {
    pub fn display_name(&self) -> &'static str {
        match self {
            SslModeUI::Disabled => "Disabled",
            SslModeUI::Preferred => "Preferred",
            SslModeUI::Required => "Required",
            SslModeUI::VerifyCa => "Verify CA",
            SslModeUI::VerifyIdentity => "Verify Identity",
        }
    }

    pub fn all_modes() -> Vec<SslModeUI> {
        vec![
            SslModeUI::Disabled,
            SslModeUI::Preferred,
            SslModeUI::Required,
            SslModeUI::VerifyCa,
            SslModeUI::VerifyIdentity,
        ]
    }
}

/// SQL Editor state
#[derive(Clone, Debug)]
pub struct SqlEditor {
    pub content: String,
    pub cursor_position: (usize, usize), // (line, column)
    pub selected_text: Option<String>,
    pub query_history: Vec<String>,
    pub history_index: Option<usize>,
    pub font_size: f32,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
}

impl SqlEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: (0, 0),
            selected_text: None,
            query_history: Vec::new(),
            history_index: None,
            font_size: 14.0,
            word_wrap: false,
            show_line_numbers: true,
        }
    }

    pub fn add_to_history(&mut self, query: String) {
        // Avoid duplicates at the end
        if self.query_history.last() != Some(&query) {
            self.query_history.push(query);
            // Keep only last 100 queries
            if self.query_history.len() > 100 {
                self.query_history.remove(0);
            }
        }
    }

    pub fn get_previous_query(&mut self) -> Option<String> {
        if self.query_history.is_empty() {
            return None;
        }

        let idx = match self.history_index {
            Some(i) if i > 0 => i - 1,
            None => self.query_history.len() - 1,
            _ => 0,
        };

        self.history_index = Some(idx);
        self.query_history.get(idx).cloned()
    }

    pub fn get_next_query(&mut self) -> Option<String> {
        match self.history_index {
            Some(i) if i + 1 < self.query_history.len() => {
                self.history_index = Some(i + 1);
                self.query_history.get(i + 1).cloned()
            }
            Some(_) => {
                self.history_index = None;
                None
            }
            None => None,
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.selected_text = None;
        self.history_index = None;
    }

    pub fn get_selected_query(&self) -> String {
        if let Some(ref selected) = self.selected_text {
            selected.clone()
        } else {
            self.content.clone()
        }
    }
}

impl Default for SqlEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the database client panel
pub fn render_database_client_panel(
    ctx: &egui::Context,
    panel: &mut DatabaseClientPanel,
    theme: &DesignTheme,
) {
    if !panel.show_panel {
        return;
    }

    egui::Window::new("🗄️ Database Client")
        .collapsible(true)
        .resizable(true)
        .default_size([900.0, 600.0])
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(35, 39, 47),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            render_database_client_content(ui, panel, theme);
        });
}

fn render_database_client_content(
    ui: &mut egui::Ui,
    panel: &mut DatabaseClientPanel,
    theme: &DesignTheme,
) {
    // Toolbar
    render_db_toolbar(ui, panel);
    ui.separator();

    if panel.connections.is_empty() {
        render_empty_state(ui, panel);
        return;
    }

    // Main content area - split into sidebar and editor
    egui::SidePanel::left("db_connections_panel")
        .resizable(true)
        .default_width(200.0)
        .width_range(150.0..=350.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(30, 33, 40),
            ..Default::default()
        })
        .show_inside(ui, |ui| {
            render_connections_list(ui, panel);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(35, 39, 47),
            ..Default::default()
        })
        .show_inside(ui, |ui| {
            render_query_editor(ui, panel, theme);
        });
}

fn render_db_toolbar(ui: &mut egui::Ui, panel: &mut DatabaseClientPanel) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("🗄️ Database Client")
                .heading()
                .color(egui::Color32::from_rgb(220, 225, 235)),
        );

        ui.separator();

        // Add connection button
        if ui
            .add(
                egui::Button::new("➕ Add Connection")
                    .fill(egui::Color32::from_rgb(64, 156, 255))
                    .min_size([120.0, 32.0].into()),
            )
            .clicked()
        {
            panel.show_add_connection_dialog = true;
            panel.new_connection_form = NewConnectionForm::default();
        }

        ui.separator();

        // Connection selector (if connections exist)
        if !panel.connections.is_empty() {
            let active_name = panel
                .get_active_connection()
                .map(|c| format!("{} {} {}@{}:{}",
                    c.db_type.icon(),
                    c.name,
                    c.username,
                    c.host,
                    c.port
                ))
                .unwrap_or_else(|| "Select connection...".to_string());

            egui::ComboBox::from_id_source("active_connection")
                .width(250.0)
                .selected_text(active_name)
                .show_ui(ui, |ui| {
                    for conn in &panel.connections {
                        let label = format!("{} {} {}",
                            conn.db_type.icon(),
                            conn.name,
                            if conn.is_connected { "●" } else { "○" }
                        );
                        if ui.selectable_label(false, &label).clicked() {
                            panel.active_connection = Some(conn.id.clone());
                        }
                    }
                });

            // Connect/Disconnect button
            if let Some(conn) = panel.get_active_connection() {
                if conn.is_connected {
                    if ui
                        .button("🔴 Disconnect")
                        .on_hover_text("Disconnect from database")
                        .clicked()
                    {
                        if let Some(c) = panel.get_active_connection_mut() {
                            c.is_connected = false;
                        }
                    }
                } else {
                    if ui
                        .button("🟢 Connect")
                        .on_hover_text("Connect to database")
                        .clicked()
                    {
                        if let Some(c) = panel.get_active_connection_mut() {
                            c.is_connected = true;
                            c.last_error = None;
                        }
                        panel.show_action_message("Connected successfully".to_string());
                    }
                }
            }
        }

        // Action message
        if let Some((ref msg, timestamp)) = panel.action_message {
            if timestamp.elapsed().as_secs() < 3 {
                ui.colored_label(egui::Color32::GREEN, msg);
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Close panel button
            if ui.button("✕").clicked() {
                panel.show_panel = false;
            }

            // Query history toggle
            let history_btn_fill = if panel.show_query_history {
                egui::Color32::from_rgb(64, 156, 255)
            } else {
                egui::Color32::from_rgb(60, 70, 85)
            };
            if ui
                .add(
                    egui::Button::new("🕐 History")
                        .fill(history_btn_fill)
                        .min_size([80.0, 28.0].into()),
                )
                .clicked()
            {
                panel.show_query_history = !panel.show_query_history;
            }
        });
    });
}

fn render_empty_state(ui: &mut egui::Ui, panel: &mut DatabaseClientPanel) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label("🗄️");
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("No Database Connections")
                    .size(18.0)
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Add a connection to start querying databases")
                    .small()
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(24.0);
            if ui
                .add(
                    egui::Button::new("➕ Add Your First Connection")
                        .fill(egui::Color32::from_rgb(64, 156, 255))
                        .min_size([200.0, 40.0].into()),
                )
                .clicked()
            {
                panel.show_add_connection_dialog = true;
            }
        });
    });
}

fn render_connections_list(ui: &mut egui::Ui, panel: &mut DatabaseClientPanel) {
    ui.label(
        egui::RichText::new("Connections")
            .strong()
            .color(egui::Color32::from_rgb(180, 190, 205)),
    );
    ui.add_space(8.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for conn in &panel.connections {
            let is_active = panel.active_connection.as_ref() == Some(&conn.id);
            let is_connected = conn.is_connected;

            let bg_color = if is_active {
                egui::Color32::from_rgb(64, 120, 200)
            } else {
                egui::Color32::TRANSPARENT
            };

            let status_color = if is_connected {
                egui::Color32::from_rgb(72, 199, 116)
            } else {
                egui::Color32::from_rgb(180, 60, 60)
            };

            let frame = egui::Frame::none()
                .fill(bg_color)
                .rounding(4.0)
                .inner_margin(egui::Margin::same(8.0));

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(conn.db_type.icon());
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&conn.name)
                                .strong()
                                .color(if is_active {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(220, 225, 235)
                                }),
                        );
                        ui.horizontal(|ui| {
                            ui.colored_label(status_color, if is_connected { "●" } else { "○" });
                            ui.label(
                                egui::RichText::new(format!("{}@{}", conn.username, conn.host))
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                    });
                });
            })
            .response
            .interact(egui::Sense::click())
            .context_menu(|ui| {
                if ui.button("Connect").clicked() {
                    if let Some(c) = panel.connections.iter_mut().find(|c| c.id == conn.id) {
                        c.is_connected = true;
                    }
                    ui.close_menu();
                }
                if ui.button("Disconnect").clicked() {
                    if let Some(c) = panel.connections.iter_mut().find(|c| c.id == conn.id) {
                        c.is_connected = false;
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete").clicked() {
                    panel.connections.retain(|c| c.id != conn.id);
                    if panel.active_connection == Some(conn.id.clone()) {
                        panel.active_connection = panel.connections.first().map(|c| c.id.clone());
                    }
                    ui.close_menu();
                }
            })
            .clicked()
            .then(|| {
                panel.active_connection = Some(conn.id.clone());
            });

            ui.add_space(4.0);
        }
    });
}

fn render_query_editor(ui: &mut egui::Ui, panel: &mut DatabaseClientPanel, theme: &DesignTheme) {
    let conn = panel.get_active_connection();

    if conn.is_none() || !conn.unwrap().is_connected {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("🔌");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("No Active Connection")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
                ui.label(
                    egui::RichText::new("Select or connect to a database to start querying")
                        .small()
                        .color(egui::Color32::GRAY),
                );
            });
        });
        return;
    }

    let conn = conn.unwrap();

    // Editor toolbar
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Query Editor - {}", conn.db_type.display_name()))
                .strong(),
        );

        ui.separator();

        // Execute button
        if ui
            .add(
                egui::Button::new("▶ Execute")
                    .fill(egui::Color32::from_rgb(72, 199, 116))
                    .min_size([100.0, 32.0].into()),
            )
            .on_hover_text("Execute query (Ctrl+Enter)")
            .clicked()
        {
            let query = panel.sql_editor.get_selected_query();
            if !query.is_empty() {
                panel.execute_query(query);
            }
        }

        // Clear button
        if ui.button("🗑 Clear").clicked() {
            panel.sql_editor.clear();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Word wrap toggle
            let wrap_text = if panel.sql_editor.word_wrap { "↩" } else { "→" };
            if ui.button(wrap_text).on_hover_text("Toggle word wrap").clicked() {
                panel.sql_editor.word_wrap = !panel.sql_editor.word_wrap;
            }

            // Line numbers toggle
            let ln_text = if panel.sql_editor.show_line_numbers { "123" } else { "lines" };
            if ui.button(ln_text).on_hover_text("Toggle line numbers").clicked() {
                panel.sql_editor.show_line_numbers = !panel.sql_editor.show_line_numbers;
            }
        });
    });

    ui.add_space(4.0);

    // SQL editor area
    let editor_height = ui.available_height() * 0.4;

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(25, 27, 32))
        .rounding(4.0)
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let text_style = egui::TextStyle::Monospace;
            let font_id = ui.style().text_styles.get(&text_style).cloned()
                .unwrap_or_else(|| egui::FontId::monospace(14.0));

            let edit = egui::TextEdit::multiline(&mut panel.sql_editor.content)
                .font(font_id)
                .desired_rows(10)
                .desired_width(if panel.sql_editor.word_wrap { available_width } else { f32::INFINITY })
                .lock_focus(true);

            ui.add_sized([available_width, editor_height], edit);
        });

    ui.add_space(8.0);

    // Results area
    render_query_results(ui, panel);
}

fn render_query_results(ui: &mut egui::Ui, panel: &DatabaseClientPanel) {
    if panel.query_results.is_empty() {
        ui.label(
            egui::RichText::new("Query Results")
                .strong()
                .color(egui::Color32::GRAY),
        );
        ui.separator();
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Execute a query to see results")
                        .color(egui::Color32::GRAY),
                );
            });
        });
        return;
    }

    // Show most recent result
    if let Some(result) = panel.query_results.last() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Query Results")
                    .strong()
                    .color(egui::Color32::from_rgb(220, 225, 235)),
            );
            ui.label(
                egui::RichText::new(format!("{} rows in {} ms", result.row_count, result.execution_time_ms))
                    .small()
                    .color(egui::Color32::GRAY),
            );
        });
        ui.separator();

        if let Some(ref error) = result.error {
            ui.colored_label(egui::Color32::from_rgb(255, 87, 87), format!("Error: {}", error));
        } else if !result.rows.is_empty() {
            // Results table
            egui::ScrollArea::both().show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    for col in &result.columns {
                        ui.add_sized(
                            [120.0, 24.0],
                            egui::Label::new(
                                egui::RichText::new(col)
                                    .strong()
                                    .color(egui::Color32::from_rgb(180, 190, 205)),
                            ),
                        );
                    }
                });
                ui.separator();

                // Rows
                for row in &result.rows {
                    ui.horizontal(|ui| {
                        for cell in row {
                            ui.add_sized(
                                [120.0, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(cell)
                                        .color(egui::Color32::from_rgb(220, 225, 235)),
                                ),
                            );
                        }
                    });
                }
            });
        } else {
            ui.label("Query executed successfully. No rows returned.");
        }
    }
}

/// Render the add connection dialog
pub fn render_add_connection_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    panel: &mut DatabaseClientPanel,
) {
    if !*show {
        return;
    }

    egui::Window::new("Add Database Connection")
        .collapsible(false)
        .resizable(false)
        .default_size([450.0, 500.0])
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(42, 48, 58),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("➕ Add Database Connection")
                        .heading()
                        .color(egui::Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(10.0);
            });
            ui.separator();

            // Database type selection
            ui.label("Database Type:");
            ui.horizontal_wrapped(|ui| {
                for db_type in DatabaseTypeUI::all_types() {
                    let is_selected = panel.selected_db_type == db_type;
                    let btn = egui::Button::new(format!("{} {}", db_type.icon(), db_type.display_name()))
                        .fill(if is_selected {
                            egui::Color32::from_rgb(64, 156, 255)
                        } else {
                            egui::Color32::from_rgb(50, 55, 65)
                        })
                        .rounding(4.0);
                    if ui.add(btn).clicked() {
                        panel.selected_db_type = db_type.clone();
                        // Auto-set default port
                        panel.new_connection_form.port = db_type.default_port().to_string();
                    }
                }
            });
            ui.add_space(8.0);
            ui.separator();

            // Connection details
            egui::Grid::new("connection_grid")
                .num_columns(2)
                .spacing([10.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Connection Name:");
                    ui.text_edit_singleline(&mut panel.new_connection_form.name);
                    ui.end_row();

                    ui.label("Host:");
                    ui.text_edit_singleline(&mut panel.new_connection_form.host);
                    ui.end_row();

                    ui.label("Port:");
                    ui.text_edit_singleline(&mut panel.new_connection_form.port);
                    ui.end_row();

                    ui.label("Database:");
                    ui.text_edit_singleline(&mut panel.new_connection_form.database);
                    ui.end_row();

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut panel.new_connection_form.username);
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut panel.new_connection_form.password).password(true));
                    ui.end_row();

                    ui.label("SSL Mode:");
                    egui::ComboBox::from_id_source("ssl_mode")
                        .width(200.0)
                        .selected_text(panel.new_connection_form.ssl_mode.display_name())
                        .show_ui(ui, |ui| {
                            for mode in SslModeUI::all_modes() {
                                if ui.selectable_label(false, mode.display_name()).clicked() {
                                    panel.new_connection_form.ssl_mode = mode;
                                }
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // SSH Tunnel option
            ui.checkbox(&mut panel.new_connection_form.use_ssh_tunnel, "Use SSH Tunnel");
            if panel.new_connection_form.use_ssh_tunnel {
                ui.label("Select SSH Server:");
                ui.text_edit_singleline(&mut panel.new_connection_form.ssh_server_id.get_or_insert_default());
            }

            ui.add_space(10.0);
            ui.separator();

            // Buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    *show = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new("Save Connection")
                                .fill(egui::Color32::from_rgb(64, 156, 255))
                                .min_size([140.0, 36.0].into()),
                        )
                        .clicked()
                    {
                        if !panel.new_connection_form.name.is_empty()
                            && !panel.new_connection_form.host.is_empty()
                        {
                            let port: u16 = panel
                                .new_connection_form
                                .port
                                .parse()
                                .unwrap_or_else(|_| panel.selected_db_type.default_port());

                            panel.add_connection(
                                panel.new_connection_form.name.clone(),
                                panel.selected_db_type.clone(),
                                panel.new_connection_form.host.clone(),
                                port,
                                panel.new_connection_form.database.clone(),
                                panel.new_connection_form.username.clone(),
                            );

                            panel.show_action_message(format!(
                                "Added {} connection: {}",
                                panel.selected_db_type.display_name(),
                                panel.new_connection_form.name
                            ));
                            *show = false;
                        }
                    }
                });
            });
        });
}

/// Render query history panel
pub fn render_query_history_panel(
    ctx: &egui::Context,
    show: &mut bool,
    panel: &mut DatabaseClientPanel,
) {
    if !*show {
        return;
    }

    egui::Window::new("Query History")
        .collapsible(true)
        .resizable(true)
        .default_size([400.0, 300.0])
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(42, 48, 58),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Query History");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        *show = false;
                    }
                    if ui.button("🗑 Clear").clicked() {
                        panel.sql_editor.query_history.clear();
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, query) in panel.sql_editor.query_history.iter().enumerate().rev() {
                    let truncated = if query.len() > 60 {
                        format!("{}...", &query[..60])
                    } else {
                        query.clone()
                    };

                    let response = ui
                        .add(
                            egui::Button::new(&truncated)
                                .fill(egui::Color32::from_rgb(50, 55, 65))
                                .min_size([ui.available_width(), 28.0].into()),
                        )
                        .on_hover_text(query);

                    if response.clicked() {
                        panel.sql_editor.content = query.clone();
                        panel.sql_editor.history_index = Some(idx);
                    }

                    if response.secondary_clicked() {
                        ui.output_mut(|o| o.copied_text = query.clone());
                    }
                }
            });
        });
}

// Re-export for external use
pub use crate::file_preview_ui::get_file_icon;
