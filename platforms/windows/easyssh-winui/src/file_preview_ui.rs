#![allow(dead_code)]

//! File Preview UI Integration for EasySSH
//!
//! This module provides UI rendering functions for the file preview system,
//! supporting multiple file types with syntax highlighting and search.

use crate::file_preview::{FilePreview, PreviewTheme};
use std::collections::HashMap;

/// File preview panel UI manager
pub struct FilePreviewPanel {
    pub show_panel: bool,
    pub preview: FilePreview,
    pub preview_history: Vec<(String, String)>, // (file_name, content)
    pub history_index: Option<usize>,
    pub file_associations: HashMap<String, String>, // extension -> language
}

impl FilePreviewPanel {
    pub fn new() -> Self {
        let mut associations = HashMap::new();
        // Common file type associations
        associations.insert("rs".to_string(), "rust".to_string());
        associations.insert("js".to_string(), "javascript".to_string());
        associations.insert("ts".to_string(), "typescript".to_string());
        associations.insert("py".to_string(), "python".to_string());
        associations.insert("java".to_string(), "java".to_string());
        associations.insert("cpp".to_string(), "cpp".to_string());
        associations.insert("c".to_string(), "c".to_string());
        associations.insert("go".to_string(), "go".to_string());
        associations.insert("rb".to_string(), "ruby".to_string());
        associations.insert("php".to_string(), "php".to_string());
        associations.insert("swift".to_string(), "swift".to_string());
        associations.insert("kt".to_string(), "kotlin".to_string());
        associations.insert("sql".to_string(), "sql".to_string());
        associations.insert("json".to_string(), "json".to_string());
        associations.insert("xml".to_string(), "xml".to_string());
        associations.insert("yaml".to_string(), "yaml".to_string());
        associations.insert("yml".to_string(), "yaml".to_string());
        associations.insert("toml".to_string(), "toml".to_string());
        associations.insert("md".to_string(), "markdown".to_string());
        associations.insert("html".to_string(), "html".to_string());
        associations.insert("css".to_string(), "css".to_string());
        associations.insert("scss".to_string(), "scss".to_string());
        associations.insert("sh".to_string(), "shell".to_string());
        associations.insert("bash".to_string(), "shell".to_string());
        associations.insert("ps1".to_string(), "powershell".to_string());
        associations.insert("dockerfile".to_string(), "dockerfile".to_string());
        associations.insert("log".to_string(), "log".to_string());
        associations.insert("csv".to_string(), "csv".to_string());

        Self {
            show_panel: false,
            preview: FilePreview::new(),
            preview_history: Vec::new(),
            history_index: None,
            file_associations: associations,
        }
    }

    pub fn open_file(&mut self, file_name: String, content: String) {
        // Add to history
        self.preview_history.push((file_name.clone(), content.clone()));
        self.history_index = Some(self.preview_history.len() - 1);

        // Open in preview
        self.preview.open(file_name, content);
        self.show_panel = true;
    }

    pub fn can_preview(&self, file_name: &str) -> bool {
        self.preview.can_preview(file_name)
    }

    pub fn navigate_history_back(&mut self) {
        if let Some(idx) = self.history_index {
            if idx > 0 {
                self.history_index = Some(idx - 1);
                if let Some((name, content)) = self.preview_history.get(idx - 1) {
                    self.preview.open(name.clone(), content.clone());
                }
            }
        }
    }

    pub fn navigate_history_forward(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.preview_history.len() {
                self.history_index = Some(idx + 1);
                if let Some((name, content)) = self.preview_history.get(idx + 1) {
                    self.preview.open(name.clone(), content.clone());
                }
            }
        }
    }

    pub fn clear_history(&mut self) {
        self.preview_history.clear();
        self.history_index = None;
        self.preview.close();
    }
}

impl Default for FilePreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the file preview side panel
pub fn render_file_preview_panel(
    ctx: &egui::Context,
    panel: &mut FilePreviewPanel,
) {
    if !panel.show_panel {
        return;
    }

    let title = if panel.preview.is_open {
        format!("📄 {}", panel.preview.file_name)
    } else {
        "📄 File Preview".to_string()
    };

    egui::SidePanel::right("file_preview_panel")
        .width_range(300.0..=800.0)
        .default_width(500.0)
        .frame(egui::Frame {
            fill: match panel.preview.theme {
                PreviewTheme::Dark => egui::Color32::from_rgb(30, 32, 38),
                PreviewTheme::Light => egui::Color32::from_rgb(250, 250, 250),
                PreviewTheme::HighContrast => egui::Color32::BLACK,
            },
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            render_preview_header(ui, panel, &title);
            ui.separator();

            if panel.preview.is_open {
                render_preview_toolbar(ui, panel);
                ui.separator();
                render_preview_content(ui, panel);
                render_preview_footer(ui, panel);
            } else {
                render_empty_preview(ui);
            }
        });
}

fn render_preview_header(ui: &mut egui::Ui, panel: &mut FilePreviewPanel, title: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .heading()
                .color(egui::Color32::from_rgb(220, 225, 235)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Close button
            if ui
                .add(
                    egui::Button::new("✕")
                        .fill(egui::Color32::from_rgb(80, 60, 60))
                        .min_size([28.0, 28.0].into()),
                )
                .clicked()
            {
                panel.show_panel = false;
            }
        });
    });
}

fn render_preview_toolbar(ui: &mut egui::Ui, panel: &mut FilePreviewPanel) {
    ui.horizontal(|ui| {
        // Navigation buttons
        let can_go_back = panel.history_index.map(|i| i > 0).unwrap_or(false);
        let can_go_forward = panel
            .history_index
            .map(|i| i + 1 < panel.preview_history.len())
            .unwrap_or(false);

        ui.add_enabled_ui(can_go_back, |ui| {
            if ui
                .button("◀")
                .on_hover_text("Previous file")
                .clicked()
            {
                panel.navigate_history_back();
            }
        });

        ui.add_enabled_ui(can_go_forward, |ui| {
            if ui
                .button("▶")
                .on_hover_text("Next file")
                .clicked()
            {
                panel.navigate_history_forward();
            }
        });

        ui.separator();

        // Search
        let mut search_query = panel.preview.search_query.clone();
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut search_query);
            if response.changed() {
                panel.preview.search(&search_query);
            }
            if !panel.preview.search_matches.is_empty() {
                ui.label(format!(
                    "{}/{}",
                    panel.preview.current_match.map(|i| i + 1).unwrap_or(0),
                    panel.preview.search_matches.len()
                ));
                if ui.button("▲").clicked() {
                    panel.preview.prev_match();
                }
                if ui.button("▼").clicked() {
                    panel.preview.next_match();
                }
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Theme toggle
            let theme_icon = match panel.preview.theme {
                PreviewTheme::Dark => "🌙",
                PreviewTheme::Light => "☀",
                PreviewTheme::HighContrast => "HC",
            };
            if ui.button(theme_icon).on_hover_text("Toggle theme").clicked() {
                panel.preview.cycle_theme();
            }

            // Line numbers toggle
            let ln_text = if panel.preview.show_line_numbers {
                "123"
            } else {
                "lines"
            };
            if ui
                .button(ln_text)
                .on_hover_text("Toggle line numbers")
                .clicked()
            {
                panel.preview.toggle_line_numbers();
            }

            // Word wrap toggle
            let wrap_text = if panel.preview.wrap_text { "↩" } else { "→" };
            if ui
                .button(wrap_text)
                .on_hover_text("Toggle word wrap")
                .clicked()
            {
                panel.preview.toggle_wrap();
            }

            // Zoom controls
            if ui.button("+").on_hover_text("Zoom in").clicked() {
                panel.preview.zoom_in();
            }
            if ui.button("-").on_hover_text("Zoom out").clicked() {
                panel.preview.zoom_out();
            }
            if ui.button("⟲").on_hover_text("Reset zoom").clicked() {
                panel.preview.reset_zoom();
            }
        });
    });
}

fn render_preview_content(ui: &mut egui::Ui, panel: &FilePreviewPanel) {
    if panel.preview.is_binary {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("📦");
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Binary file")
                        .color(egui::Color32::GRAY)
                        .size(16.0),
                );
                ui.label(
                    egui::RichText::new(&panel.preview.file_name)
                        .color(egui::Color32::GRAY)
                        .size(12.0),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Size: {} bytes",
                        panel.preview.char_count
                    ))
                    .small()
                    .color(egui::Color32::GRAY),
                );
            });
        });
        return;
    }

    let text_color = match panel.preview.theme {
        PreviewTheme::Dark => egui::Color32::from_rgb(220, 225, 235),
        PreviewTheme::Light => egui::Color32::from_rgb(30, 30, 30),
        PreviewTheme::HighContrast => egui::Color32::WHITE,
    };

    let bg_color = match panel.preview.theme {
        PreviewTheme::Dark => egui::Color32::from_rgb(25, 27, 32),
        PreviewTheme::Light => egui::Color32::from_rgb(255, 255, 255),
        PreviewTheme::HighContrast => egui::Color32::BLACK,
    };

    // Get content to display (truncated if too large)
    let content = panel.preview.truncated_content();

    egui::Frame::none()
        .fill(bg_color)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_source("preview_scroll")
                .show(ui, |ui| {
                    let font_id =
                        egui::FontId::monospace(panel.preview.font_size);

                    let mut text_format = egui::TextFormat::default();
                    text_format.font_id = font_id;
                    text_format.color = text_color;

                    if panel.preview.wrap_text {
                        ui.with_layout(
                            egui::Layout::left_to_right(egui::Align::TOP)
                                .with_main_wrap(true),
                            |ui| {
                                if panel.preview.show_line_numbers {
                                    render_with_line_numbers(
                                        ui,
                                        &content,
                                        &panel.preview,
                                        text_color,
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new(&content)
                                            .font(font_id)
                                            .color(text_color),
                                    );
                                }
                            },
                        );
                    } else {
                        ui.horizontal(|ui| {
                            if panel.preview.show_line_numbers {
                                render_with_line_numbers(
                                    ui,
                                    &content,
                                    &panel.preview,
                                    text_color,
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(&content)
                                        .font(font_id)
                                        .color(text_color),
                                );
                            }
                        });
                    }
                });
        });
}

fn render_with_line_numbers(
    ui: &mut egui::Ui,
    content: &str,
    preview: &FilePreview,
    text_color: egui::Color32,
) {
    let line_number_color = egui::Color32::from_rgb(100, 110, 130);
    let font_id = egui::FontId::monospace(preview.font_size);

    // Calculate line number width
    let total_lines = content.lines().count();
    let line_num_width = format!("{}", total_lines).len().max(3);

    // Use a table-like layout
    egui::Grid::new("preview_with_linenumbers")
        .num_columns(2)
        .spacing([8.0, 2.0])
        .show(ui, |ui| {
            let mut current_match_idx = 0;

            for (line_idx, line) in content.lines().enumerate() {
                // Check if this line has a search match
                let has_match = preview.search_matches.iter().enumerate().any(
                    |(idx, (match_line, _))| {
                        *match_line == line_idx && {
                            if preview.current_match == Some(idx) {
                                current_match_idx = idx;
                                true
                            } else {
                                false
                            }
                        }
                    },
                );

                // Line number
                let line_num = format!("{:>width$}", line_idx + 1, width = line_num_width);
                ui.label(
                    egui::RichText::new(line_num)
                        .font(font_id.clone())
                        .color(line_number_color),
                );

                // Line content
                if has_match && preview.current_match == Some(current_match_idx) {
                    // Highlight current match line
                    ui.colored_label(egui::Color32::from_rgb(60, 80, 120), line);
                } else {
                    ui.label(
                        egui::RichText::new(line)
                            .font(font_id.clone())
                            .color(text_color),
                    );
                }

                ui.end_row();
            }
        });
}

fn render_preview_footer(ui: &mut egui::Ui, panel: &FilePreviewPanel) {
    ui.separator();
    ui.horizontal(|ui| {
        let summary = panel.preview.get_preview_summary();
        ui.label(
            egui::RichText::new(summary)
                .small()
                .color(egui::Color32::GRAY),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if panel.preview.is_too_large() {
                ui.label(
                    egui::RichText::new("⚠ Large file - preview truncated")
                        .small()
                        .color(egui::Color32::YELLOW),
                );
            }
        });
    });
}

fn render_empty_preview(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label("📄");
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("No file selected")
                    .size(16.0)
                    .color(egui::Color32::GRAY),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Select a file from the SFTP browser to preview")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        });
    });
}

/// Quick preview dialog for file browsing
pub fn render_quick_preview(
    ctx: &egui::Context,
    show: &mut bool,
    file_name: &str,
    content: &str,
) {
    if !*show {
        return;
    }

    let title = format!("Preview: {}", file_name);

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_size([600.0, 400.0])
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(35, 39, 47),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} lines", content.lines().count()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        *show = false;
                    }
                    if ui.button("📋 Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = content.to_string());
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut content.to_string())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(20),
                );
            });
        });
}

/// File type icon helper
pub fn get_file_icon(file_name: &str) -> &'static str {
    let ext = file_name
        .split('.')
        .next_back()
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "rs" => "🦀",
        "js" | "jsx" | "ts" | "tsx" => "📜",
        "py" => "🐍",
        "java" => "☕",
        "go" => "🐹",
        "rb" => "💎",
        "php" => "🐘",
        "swift" => "🦉",
        "kt" => "🎯",
        "cpp" | "c" | "h" | "hpp" => "⚙",
        "html" | "htm" | "xhtml" => "🌐",
        "css" | "scss" | "sass" | "less" => "🎨",
        "json" | "xml" => "📋",
        "sql" => "🗄",
        "md" | "markdown" => "📝",
        "txt" => "📄",
        "log" => "📊",
        "csv" => "📈",
        "yaml" | "yml" | "toml" | "ini" | "cfg" => "⚙",
        "sh" | "bash" | "zsh" | "ps1" => "🖥",
        "dockerfile" => "🐳",
        "gitignore" => "🌲",
        "lock" => "🔒",
        _ => "📄",
    }
}

/// Check if file is likely a text file based on extension
pub fn is_text_file(file_name: &str) -> bool {
    let preview = FilePreview::new();
    preview.can_preview(file_name)
}
