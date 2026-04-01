#![allow(dead_code)]

//! Hotkey Helper Functions for EasySSH Windows
//!
//! This module provides helper functions for the professional hotkey system.
//! These are called from main.rs to handle hotkey actions.

use crate::{EasySSHApp, SessionTab};
use crate::hotkeys::{HotkeyAction, Command};
use eframe::egui;
use tracing::{error, info};

impl EasySSHApp {
    /// Save hotkey configuration to file
    pub fn save_hotkey_config(&self) {
        if let Ok(manager) = self.hotkey_manager.lock() {
            if let Err(e) = manager.save_to_file() {
                error!("Failed to save hotkey config: {}", e);
            } else {
                info!("Hotkey configuration saved successfully");
            }
        }
    }

    /// Process all hotkeys - called from update()
    pub fn process_hotkeys(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Command palette has priority
        if self.command_palette.visible {
            self.command_palette.handle_input(ctx);
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.command_palette.hide();
            }
            return;
        }

        // Hotkey settings recording mode
        if self.show_hotkey_settings && self.hotkey_settings.recording_binding {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_hotkey_settings = false;
            }
            return;
        }

        // Process registered hotkeys
        let triggered = if let Ok(mut mgr) = self.hotkey_manager.lock() {
            mgr.process_input(ctx)
        } else { vec![] };

        for action in triggered {
            self.execute_hotkey_action(action, ctx, frame);
        }

        // Terminal: Ctrl+C SIGINT
        if self.is_terminal_active && self.current_session_id.is_some() {
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::C)) {
                self.stop_current_command();
            }
        }

        // Terminal font zoom
        if self.is_terminal_active {
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Plus)) {
                self.terminal_font_zoom = (self.terminal_font_zoom + 0.1).clamp(0.5, 3.0);
            } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Minus)) {
                self.terminal_font_zoom = (self.terminal_font_zoom - 0.1).clamp(0.5, 3.0);
            } else if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
                self.terminal_font_zoom = 1.0;
            }
        }

        // Tab switching
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab)) {
            self.switch_to_next_tab();
        } else if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab)) {
            self.switch_to_prev_tab();
        }

        // F11 fullscreen
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.toggle_fullscreen(frame);
        }

        // F12 toggle performance monitor
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            self.show_performance_panel = !self.show_performance_panel;
        }

        // Ctrl+K Command Palette
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) && !self.is_terminal_active {
            self.open_command_palette();
        }

        // Ctrl+Shift+F Global Search
        if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F)) {
            self.open_global_search();
        }

        // Ctrl+T New Tab
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::T)) {
            self.new_tab();
        }

        // Ctrl+W Close Tab
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
            self.close_current_tab();
        }

        // Ctrl+1..9 Switch tabs
        for idx in 0..9 {
            let key = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                      egui::Key::Num4, egui::Key::Num5, egui::Key::Num6,
                      egui::Key::Num7, egui::Key::Num8, egui::Key::Num9][idx];
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(key)) {
                self.switch_to_tab_by_index(idx);
                break;
            }
        }

        // Ctrl+Shift+/ Shortcut Cheatsheet
        if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Slash)) {
            self.show_shortcut_cheatsheet = !self.show_shortcut_cheatsheet;
        }

        // Escape close overlays
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.theme_gallery.is_open { self.theme_gallery.close(); }
            else if self.theme_editor.is_open { self.theme_editor.close(); }
            else if self.settings_panel.is_open { self.settings_panel.close(); }
            else if self.show_global_search { self.show_global_search = false; }
            else if self.show_add_dialog { self.show_add_dialog = false; }
            else if self.show_connect_dialog { self.show_connect_dialog = false; }
            else if self.show_hotkey_settings { self.show_hotkey_settings = false; }
            else if self.show_shortcut_cheatsheet { self.show_shortcut_cheatsheet = false; }
            else if self.show_performance_panel { self.show_performance_panel = false; }
        }
    }

    /// Execute a hotkey action
    pub fn execute_hotkey_action(&mut self, action: HotkeyAction, _ctx: &egui::Context, frame: &mut eframe::Frame) {
        match action {
            HotkeyAction::QuickConnectLast => {
                let last = self.hotkey_manager.lock().ok().and_then(|m| m.get_last_server().cloned());
                if let Some(id) = last {
                    if let Some(srv) = self.servers.iter().find(|s| s.id == id).cloned() {
                        self.connect_server = Some(srv);
                        self.show_connect_dialog = true;
                    }
                }
            }
            HotkeyAction::NewConnectionWindow => info!("New connection window requested"),
            HotkeyAction::NewTab => self.new_tab(),
            HotkeyAction::CloseTab => self.close_current_tab(),
            HotkeyAction::NextTab => self.switch_to_next_tab(),
            HotkeyAction::PrevTab => self.switch_to_prev_tab(),
            HotkeyAction::SwitchTab1 => self.switch_to_tab_by_index(0),
            HotkeyAction::SwitchTab2 => self.switch_to_tab_by_index(1),
            HotkeyAction::SwitchTab3 => self.switch_to_tab_by_index(2),
            HotkeyAction::SwitchTab4 => self.switch_to_tab_by_index(3),
            HotkeyAction::SwitchTab5 => self.switch_to_tab_by_index(4),
            HotkeyAction::SwitchTab6 => self.switch_to_tab_by_index(5),
            HotkeyAction::SwitchTab7 => self.switch_to_tab_by_index(6),
            HotkeyAction::SwitchTab8 => self.switch_to_tab_by_index(7),
            HotkeyAction::SwitchTab9 => self.switch_to_tab_by_index(8),
            HotkeyAction::CommandPalette => self.open_command_palette(),
            HotkeyAction::GlobalSearch => self.open_global_search(),
            HotkeyAction::ToggleFullscreen => self.toggle_fullscreen(frame),
            HotkeyAction::TerminalZoomIn => self.terminal_font_zoom = (self.terminal_font_zoom + 0.1).clamp(0.5, 3.0),
            HotkeyAction::TerminalZoomOut => self.terminal_font_zoom = (self.terminal_font_zoom - 0.1).clamp(0.5, 3.0),
            HotkeyAction::TerminalZoomReset => self.terminal_font_zoom = 1.0,
            HotkeyAction::TerminalClear => self.terminal_output.clear(),
            HotkeyAction::FocusServers => info!("Focus servers list"),
            HotkeyAction::FocusTerminal => self.is_terminal_active = true,
            HotkeyAction::FocusFileBrowser => self.show_file_browser = true,
            HotkeyAction::ToggleSidebar => info!("Toggle sidebar"),
            HotkeyAction::OpenSnippets => self.show_snippets_panel = !self.show_snippets_panel,
            HotkeyAction::InsertSnippet => { if !self.session_tabs.is_empty() { self.show_snippets_panel = true; } }
            HotkeyAction::SplitHorizontal => self.split_panel_horizontal(),
            HotkeyAction::SplitVertical => self.split_panel_vertical(),
            HotkeyAction::ClosePanel => self.close_current_tab(),
            HotkeyAction::NextPanel => self.switch_to_next_tab(),
            HotkeyAction::PrevPanel => self.switch_to_prev_tab(),
            HotkeyAction::TerminalCopy => {
                // Copy from terminal - handled contextually in terminal component
                info!("Terminal copy requested");
            }
            HotkeyAction::TerminalPaste => {
                // Paste to terminal - handled contextually in terminal component
                info!("Terminal paste requested");
            }
            HotkeyAction::Custom(_) => {}
        }
    }

    /// Open the command palette
    pub fn open_command_palette(&mut self) {
        self.setup_command_palette();
        self.command_palette.show();
    }

    /// Setup commands for the palette
    pub fn setup_command_palette(&mut self) {
        self.command_palette.commands.clear();
        let actions = vec![
            HotkeyAction::NewTab, HotkeyAction::CloseTab, HotkeyAction::CommandPalette,
            HotkeyAction::GlobalSearch, HotkeyAction::ToggleFullscreen, HotkeyAction::TerminalClear,
        ];
        for action in actions {
            let cmd = Command {
                id: format!("{:?}", action), action: action.clone(),
                label: action.display_name(), category: action.category().to_string(),
                description: Some(format!("Category: {}", action.category())),
                icon: Some("⚡".to_string()), shortcut: action.default_hotkey(),
                execute: std::sync::Arc::new(move || info!("Command: {:?}", action)),
            };
            self.command_palette.register_command(cmd);
        }
    }

    /// Open global search
    pub fn open_global_search(&mut self) {
        self.show_global_search = true;
        self.global_search_query.clear();
        self.global_search_results.clear();
        self.global_search_selected = Some(0);
    }

    /// Create a new tab
    pub fn new_tab(&mut self) {
        let id = uuid::Uuid::new_v4().to_string();
        self.session_tabs.push(SessionTab {
            session_id: id.clone(), server_id: String::new(),
            title: "New Tab".to_string(), output: String::new(),
            input: String::new(), connected: false,
        });
        self.active_tab = Some(id);
    }

    /// Close current tab
    pub fn close_current_tab(&mut self) {
        if let Some(active) = &self.active_tab {
            if let Some(session_id) = &self.current_session_id {
                if session_id == active {
                    let _ = self.view_model.lock().unwrap().disconnect(session_id);
                    self.current_session_id = None;
                }
            }
            if let Some(idx) = self.session_tabs.iter().position(|t| &t.session_id == active) {
                self.session_tabs.remove(idx);
                if !self.session_tabs.is_empty() {
                    let new_idx = idx.min(self.session_tabs.len() - 1);
                    self.active_tab = Some(self.session_tabs[new_idx].session_id.clone());
                    self.terminal_output = self.session_tabs[new_idx].output.clone();
                } else {
                    self.active_tab = None;
                    self.terminal_output.clear();
                }
            }
        }
    }

    /// Switch to next tab
    pub fn switch_to_next_tab(&mut self) {
        if let Some(active) = &self.active_tab {
            if let Some(idx) = self.session_tabs.iter().position(|t| &t.session_id == active) {
                let next = (idx + 1) % self.session_tabs.len();
                self.active_tab = Some(self.session_tabs[next].session_id.clone());
                self.terminal_output = self.session_tabs[next].output.clone();
            }
        } else if !self.session_tabs.is_empty() {
            self.active_tab = Some(self.session_tabs[0].session_id.clone());
        }
    }

    /// Switch to previous tab
    pub fn switch_to_prev_tab(&mut self) {
        if let Some(active) = &self.active_tab {
            if let Some(idx) = self.session_tabs.iter().position(|t| &t.session_id == active) {
                let prev = if idx == 0 { self.session_tabs.len() - 1 } else { idx - 1 };
                self.active_tab = Some(self.session_tabs[prev].session_id.clone());
                self.terminal_output = self.session_tabs[prev].output.clone();
            }
        } else if !self.session_tabs.is_empty() {
            let last = self.session_tabs.len() - 1;
            self.active_tab = Some(self.session_tabs[last].session_id.clone());
        }
    }

    /// Switch to tab by index
    pub fn switch_to_tab_by_index(&mut self, idx: usize) {
        if let Some(tab) = self.session_tabs.get(idx) {
            self.active_tab = Some(tab.session_id.clone());
            self.current_session_id = Some(tab.session_id.clone());
            self.terminal_output = tab.output.clone();
        }
    }

    /// Toggle fullscreen mode
    pub fn toggle_fullscreen(&mut self, _frame: &mut eframe::Frame) {
        self.is_fullscreen = !self.is_fullscreen;
        // Note: Fullscreen API is not available in egui 0.28
        // This would require platform-specific window handling
    }

    // ==================== Split Layout Helpers ====================

    /// Split current panel horizontally
    pub fn split_panel_horizontal(&mut self) {
        if let Some(active_panel) = self.split_layout_manager.active_panel() {
            let new_id = self.split_layout_manager.split_horizontal(
                active_panel,
                crate::split_layout::PanelType::Terminal,
                "Terminal"
            );
            if let Some(id) = new_id {
                info!("Split panel horizontally, new panel: {:?}", id);
                self.split_layout_manager.set_active_panel(id);
            }
        }
    }

    /// Split current panel vertically
    pub fn split_panel_vertical(&mut self) {
        if let Some(active_panel) = self.split_layout_manager.active_panel() {
            let new_id = self.split_layout_manager.split_vertical(
                active_panel,
                crate::split_layout::PanelType::Terminal,
                "Terminal"
            );
            if let Some(id) = new_id {
                info!("Split panel vertically, new panel: {:?}", id);
                self.split_layout_manager.set_active_panel(id);
            }
        }
    }

    /// Close current panel
    pub fn close_current_panel(&mut self) {
        if let Some(active_panel) = self.split_layout_manager.active_panel() {
            self.split_layout_manager.close_panel(active_panel);
            info!("Closed panel: {:?}", active_panel);
        }
    }

    /// Switch to next panel
    pub fn switch_to_next_panel(&mut self) {
        let panels = self.split_layout_manager.all_panels();
        if let Some(active) = self.split_layout_manager.active_panel() {
            if let Some(idx) = panels.iter().position(|&p| p == active) {
                let next = (idx + 1) % panels.len();
                self.split_layout_manager.set_active_panel(panels[next]);
            }
        } else if !panels.is_empty() {
            self.split_layout_manager.set_active_panel(panels[0]);
        }
    }

    /// Switch to previous panel
    pub fn switch_to_prev_panel(&mut self) {
        let panels = self.split_layout_manager.all_panels();
        if let Some(active) = self.split_layout_manager.active_panel() {
            if let Some(idx) = panels.iter().position(|&p| p == active) {
                let prev = if idx == 0 { panels.len() - 1 } else { idx - 1 };
                self.split_layout_manager.set_active_panel(panels[prev]);
            }
        } else if !panels.is_empty() {
            let last = panels.len() - 1;
            self.split_layout_manager.set_active_panel(panels[last]);
        }
    }

    /// Render a panel based on its type
    pub fn render_panel_content(&mut self, ui: &mut egui::Ui, panel_id: crate::split_layout::PanelId, content: &crate::split_layout::PanelContent, is_active: bool) {
        use crate::split_layout::PanelType;

        // Panel title bar
        ui.horizontal(|ui| {
            ui.label(content.panel_type.icon());
            ui.label(egui::RichText::new(&content.title).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("×").clicked() {
                    self.split_layout_manager.close_panel(panel_id);
                }
            });
        });
        ui.separator();

        // Panel content based on type
        match content.panel_type {
            PanelType::Terminal => {
                if self.is_terminal_active && self.current_session_id.is_some() {
                    self.render_terminal_content(ui);
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No active terminal session");
                    });
                }
            }
            PanelType::SftpBrowser => {
                ui.label("SFTP Browser Panel");
                // SFTP content would go here
            }
            PanelType::Monitor => {
                ui.label("Monitor Panel");
                // Monitor content would go here
            }
            PanelType::ServerList => {
                ui.label("Server List Panel");
                // Server list content would go here
            }
        }
    }

    /// Render terminal content in a panel
    fn render_terminal_content(&mut self, ui: &mut egui::Ui) {
        use egui::Color32;

        let term_bg = self.theme_manager.current_theme.palette.background;
        let term_fg = self.theme_manager.current_theme.palette.foreground;
        let term_cursor = self.theme_manager.current_theme.palette.cursor;

        // Session tabs
        if !self.session_tabs.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (idx, tab) in self.session_tabs.iter().enumerate() {
                    let is_active = self.active_tab.as_ref() == Some(&tab.session_id);
                    let shortcut = if idx < 9 { format!(" Ctrl+{}", idx + 1) } else { String::new() };
                    let label = format!("{}{}{}", if is_active { "●" } else { "○" }, tab.title, shortcut);

                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .color(if is_active { Color32::WHITE } else { Color32::from_rgb(150, 158, 175) })
                    )
                    .fill(if is_active { Color32::from_rgb(50, 80, 70) } else { Color32::from_rgb(45, 50, 58) })
                    .rounding(4.0);

                    if ui.add(btn).clicked() {
                        self.active_tab = Some(tab.session_id.clone());
                        self.current_session_id = Some(tab.session_id.clone());
                        self.terminal_output = tab.output.clone();
                    }
                    ui.add_space(6.0);
                }
            });
            ui.separator();
        }

        // Terminal output
        let available_height = ui.available_height() - 50.0;
        egui::Frame {
            fill: term_bg,
            rounding: egui::Rounding::same(4.0),
            stroke: egui::Stroke::new(1.0, term_fg.linear_multiply(0.3)),
            ..Default::default()
        }.show(ui, |ui| {
            ui.set_min_height(available_height);
            ui.set_max_height(available_height);

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&self.terminal_output)
                        .monospace()
                        .size(14.0 * self.terminal_font_zoom)
                        .color(term_fg));
                });
        });

        ui.separator();

        // Command input
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("❯").color(term_cursor));

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.navigate_history(true);
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.navigate_history(false);
            }

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.command_input)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width() - 100.0),
            );

            if enter_pressed && !self.command_input.is_empty() {
                self.execute_command();
            }

            ui.memory_mut(|m| m.request_focus(response.id));

            if ui.add(egui::Button::new("Execute").min_size([80.0, 36.0].into())).clicked() {
                self.execute_command();
            }
        });
    }
}
