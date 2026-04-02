#![allow(dead_code)]

//! Snippets UI Integration for EasySSH
//!
//! This module provides UI rendering functions for the snippets system,
//! designed to be used with the main EasySSHApp.

use crate::snippets::{
    Snippet, SnippetCategory, SnippetInputDialog, SnippetManager, SnippetVariable,
};

/// Renders the snippets side panel
pub fn render_snippets_panel(
    ctx: &egui::Context,
    show_panel: &mut bool,
    manager: &mut SnippetManager,
    selected_category: &mut Option<SnippetCategory>,
    search: &mut String,
    action_message: &mut Option<(String, std::time::Instant)>,
    show_add_dialog: &mut bool,
    snippet_input_dialog: &mut SnippetInputDialog,
    command_input: &mut String,
    new_snippet_form: &mut super::NewSnippetForm,
) {
    if !*show_panel {
        return;
    }

    egui::SidePanel::left("snippets_panel")
        .width_range(200.0..=300.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(35, 39, 47),
            ..Default::default()
        })
        .show(ctx, |ui| {
            render_snippets_header(ui, show_panel, show_add_dialog, new_snippet_form);
            render_category_filters(ui, selected_category, manager);
            render_snippets_search(ui, search, manager);
            render_action_message(ui, action_message);
            render_snippets_list(
                ui,
                manager,
                snippet_input_dialog,
                command_input,
                action_message,
            );
            render_snippets_footer(ui, show_panel);
        });
}

fn render_snippets_header(
    ui: &mut egui::Ui,
    _show_panel: &mut bool,
    show_add_dialog: &mut bool,
    new_snippet_form: &mut super::NewSnippetForm,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("📝").size(18.0));
        ui.label(
            egui::RichText::new("Snippets")
                .heading()
                .color(egui::Color32::from_rgb(220, 225, 235)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new("+")
                        .fill(egui::Color32::from_rgb(64, 156, 255))
                        .min_size([28.0, 28.0].into()),
                )
                .clicked()
            {
                *show_add_dialog = true;
                *new_snippet_form = super::NewSnippetForm::default();
            }
        });
    });
    ui.add_space(8.0);
    ui.separator();
}

fn render_category_filters(
    ui: &mut egui::Ui,
    selected_category: &mut Option<SnippetCategory>,
    manager: &mut SnippetManager,
) {
    ui.horizontal_wrapped(|ui| {
        let categories: Vec<(&str, Option<SnippetCategory>)> = vec![
            ("All", None),
            ("Frequent", Some(SnippetCategory::FrequentlyUsed)),
            ("Custom", Some(SnippetCategory::Custom)),
            ("Team", Some(SnippetCategory::Team)),
        ];

        for (label, cat) in categories {
            let is_selected = *selected_category == cat;
            let btn = egui::Button::new(egui::RichText::new(label).size(12.0))
                .fill(if is_selected {
                    egui::Color32::from_rgb(64, 156, 255)
                } else {
                    egui::Color32::from_rgb(50, 55, 65)
                })
                .rounding(4.0);
            if ui.add(btn).clicked() {
                *selected_category = cat.clone();
                manager.set_category(cat);
            }
        }
    });
    ui.add_space(6.0);
}

fn render_snippets_search(ui: &mut egui::Ui, search: &mut String, manager: &mut SnippetManager) {
    ui.horizontal(|ui| {
        ui.label("🔍");
        let response = ui.text_edit_singleline(search);
        if response.changed() {
            manager.set_search_query(search.clone());
        }
    });
    ui.add_space(4.0);
    ui.separator();
}

fn render_action_message(
    ui: &mut egui::Ui,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    if let Some((ref msg, timestamp)) = action_message {
        if timestamp.elapsed().as_secs() < 3 {
            ui.colored_label(egui::Color32::GREEN, msg);
            ui.add_space(4.0);
        }
    }
}

fn render_snippets_list(
    ui: &mut egui::Ui,
    manager: &mut SnippetManager,
    snippet_input_dialog: &mut SnippetInputDialog,
    command_input: &mut String,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // Collect snippet data to avoid borrow issues
        let snippets_data: Vec<(String, String, SnippetCategory, Vec<String>, String)> = manager
            .filtered_snippets()
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    s.name.clone(),
                    s.category.clone(),
                    s.tags.clone(),
                    s.content.clone(),
                )
            })
            .collect();

        if snippets_data.is_empty() {
            ui.label(
                egui::RichText::new("No snippets found")
                    .color(egui::Color32::GRAY)
                    .size(12.0),
            );
        } else {
            for (id, name, category, tags, content) in snippets_data {
                render_snippet_item(
                    ui,
                    &id,
                    &name,
                    category,
                    &tags,
                    &content,
                    manager,
                    snippet_input_dialog,
                    command_input,
                    action_message,
                );
            }
        }
    });
}

fn render_snippet_item(
    ui: &mut egui::Ui,
    snippet_id: &str,
    snippet_name: &str,
    snippet_category: SnippetCategory,
    snippet_tags: &[String],
    snippet_content: &str,
    manager: &mut SnippetManager,
    snippet_input_dialog: &mut SnippetInputDialog,
    command_input: &mut String,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    let is_team = snippet_category == SnippetCategory::Team;
    let icon = if is_team { "👥" } else { "▶" };
    let btn_text = format!("{} {}", icon, snippet_name);
    let btn = egui::Button::new(
        egui::RichText::new(&btn_text)
            .color(egui::Color32::from_rgb(200, 210, 220))
            .size(12.0),
    )
    .fill(egui::Color32::from_rgb(45, 50, 58))
    .rounding(4.0)
    .min_size([ui.available_width(), 32.0].into());
    let response = ui.add(btn);

    if response.clicked() {
        insert_snippet(
            snippet_id,
            snippet_name,
            snippet_content,
            snippet_category.clone(),
            manager,
            snippet_input_dialog,
            command_input,
            action_message,
        );
    }

    response.context_menu(|ui| {
        if ui.button("▶ Insert").clicked() {
            insert_snippet(
                snippet_id,
                snippet_name,
                snippet_content,
                snippet_category.clone(),
                manager,
                snippet_input_dialog,
                command_input,
                action_message,
            );
            ui.close_menu();
        }
        if ui.button("📋 Copy").clicked() {
            ui.output_mut(|o| o.copied_text = snippet_content.to_string());
            *action_message = Some((
                format!("Copied: {}", snippet_name),
                std::time::Instant::now(),
            ));
            ui.close_menu();
        }
        if snippet_category == SnippetCategory::Custom || snippet_category == SnippetCategory::Team
        {
            ui.separator();
            if ui.button("🗑 Delete").clicked() {
                manager.delete_snippet(snippet_id);
                *action_message = Some((
                    format!("Deleted: {}", snippet_name),
                    std::time::Instant::now(),
                ));
                ui.close_menu();
            }
        }
    });

    response.on_hover_ui(|ui| {
        ui.set_max_width(300.0);
        ui.label(egui::RichText::new(snippet_name).strong());
        // Note: description not passed to avoid complexity, can be added if needed
        ui.separator();
        ui.label(
            egui::RichText::new(snippet_content)
                .monospace()
                .size(11.0)
                .color(egui::Color32::from_rgb(100, 180, 255)),
        );
        if !snippet_tags.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Tags: {}", snippet_tags.join(", ")))
                    .size(10.0)
                    .color(egui::Color32::GRAY),
            );
        }
    });
}

fn insert_snippet(
    snippet_id: &str,
    snippet_name: &str,
    snippet_content: &str,
    _snippet_category: SnippetCategory,
    manager: &mut SnippetManager,
    snippet_input_dialog: &mut SnippetInputDialog,
    command_input: &mut String,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    // Check if snippet has variables by looking for {{variable}} patterns
    let has_vars = snippet_content.contains("{{") && snippet_content.contains("}}");
    if !has_vars {
        *command_input = snippet_content.to_string();
        *action_message = Some((
            format!("Inserted: {}", snippet_name),
            std::time::Instant::now(),
        ));
    } else {
        *snippet_input_dialog = SnippetInputDialog::from_fields(snippet_name, snippet_content);
    }
    manager.record_usage(snippet_id);
}

fn render_snippets_footer(ui: &mut egui::Ui, show_panel: &mut bool) {
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("📥 Import").clicked() {
            // Import handled at app level
        }
        if ui.button("📤 Export").clicked() {
            // Export handled at app level
        }
        if ui.button("✕").clicked() {
            *show_panel = false;
        }
    });
}

/// Render the snippet variable input dialog
pub fn render_snippet_dialog(
    ctx: &egui::Context,
    dialog: &mut SnippetInputDialog,
    command_input: &mut String,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    if !dialog.visible {
        return;
    }

    // Pre-compute current variable info to avoid borrow issues
    let current_var_info = dialog.current_variable().map(|var| {
        let key = match var {
            SnippetVariable::Hostname => "hostname".to_string(),
            SnippetVariable::Username => "username".to_string(),
            SnippetVariable::Port => "port".to_string(),
            SnippetVariable::Password => "password".to_string(),
            SnippetVariable::Custom { name, .. } => name.clone(),
        };
        let display_name = var.display_name();
        let default_value = dialog.current_default().unwrap_or_default();
        let is_last = dialog.current_variable_idx + 1 >= dialog.variables.len();
        (key, display_name, default_value, is_last, var.clone())
    });

    egui::Window::new(format!("Insert: {}", dialog.snippet_name))
        .collapsible(false)
        .resizable(false)
        .default_size([350.0, 200.0])
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(42, 48, 58),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(&dialog.snippet_content)
                        .monospace()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 160, 180)),
                );
                ui.add_space(15.0);
            });

            if let Some((key, display_name, default_value, is_last, _var)) = current_var_info {
                ui.label(format!("Enter value for {}:", display_name));
                let mut value = dialog.values.get(&key).cloned().unwrap_or(default_value);
                let response = ui.text_edit_singleline(&mut value);
                dialog.values.insert(key.clone(), value.clone());
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("Cancel").min_size([80.0, 44.0].into()))
                        .clicked()
                    {
                        dialog.reset();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if is_last { "Insert" } else { "Next" };
                        let should_insert = ui
                            .add(egui::Button::new(btn_text).min_size([100.0, 44.0].into()))
                            .clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if should_insert {
                            let val = dialog.values.get(&key).cloned().unwrap_or_default();
                            let done = dialog.set_current_value(val);
                            if done {
                                let final_cmd = dialog.render_command();
                                *command_input = final_cmd;
                                *action_message = Some((
                                    format!("Inserted: {}", dialog.snippet_name),
                                    std::time::Instant::now(),
                                ));
                                dialog.reset();
                            }
                        }
                    });
                });
            } else {
                ui.label("No variables required.");
                if ui.button("Close").clicked() {
                    dialog.reset();
                }
            }
        });
}

/// Render the add new snippet dialog
pub fn render_add_snippet_dialog(
    ctx: &egui::Context,
    show_dialog: &mut bool,
    form: &mut super::NewSnippetForm,
    manager: &mut SnippetManager,
    action_message: &mut Option<(String, std::time::Instant)>,
) {
    if !*show_dialog {
        return;
    }

    egui::Window::new("Add New Snippet")
        .collapsible(false).resizable(false).default_size([400.0, 350.0])
        .frame(egui::Frame { fill: egui::Color32::from_rgb(42, 48, 58), stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)), ..Default::default() })
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("➕ Add New Snippet").heading().color(egui::Color32::from_rgb(220, 225, 235)));
            ui.add_space(10.0);
            ui.separator();
            egui::Grid::new("add_snippet_grid").num_columns(2).spacing([10.0, 12.0]).show(ui, |ui| {
                ui.label("Name:"); ui.text_edit_singleline(&mut form.name); ui.end_row();
                ui.label("Command:"); ui.add(egui::TextEdit::multiline(&mut form.content).desired_rows(3)); ui.end_row();
                ui.label("Description:"); ui.text_edit_singleline(&mut form.description); ui.end_row();
                ui.label("Category:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut form.category, SnippetCategory::Custom, "Custom");
                    ui.radio_value(&mut form.category, SnippetCategory::Team, "Team");
                });
                ui.end_row();
                ui.label("Tags:"); ui.text_edit_singleline(&mut form.tags).on_hover_text("Comma-separated tags"); ui.end_row();
            });
            ui.add_space(10.0);
            ui.separator();
            ui.label(egui::RichText::new("Variables: {{hostname}}, {{username}}, {{port}}, {{password}}, {{custom|default}}").small().color(egui::Color32::GRAY));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() { *show_dialog = false; }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("Add Snippet").min_size([120.0, 44.0].into())).clicked()
                        && !form.name.is_empty() && !form.content.is_empty() {
                            let tags: Vec<String> = form.tags.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                            let mut snippet = Snippet::new(form.name.clone(), form.content.clone())
                                .with_category(form.category.clone())
                                .with_tags(tags);
                            if !form.description.is_empty() {
                                snippet = snippet.with_description(form.description.clone());
                            }
                            manager.add_snippet(snippet);
                            *show_dialog = false;
                            *action_message = Some((format!("Added snippet: {}", form.name), std::time::Instant::now()));
                            *form = super::NewSnippetForm::default();
                        }
                });
            });
        });
}
