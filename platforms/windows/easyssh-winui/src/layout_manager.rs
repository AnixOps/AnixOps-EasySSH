#![allow(dead_code)]

//! Split Panel Layout Manager for EasySSH
//! Integrates the split_layout module with the main application

use crate::split_layout::{LayoutNode, PanelContent, PanelId, PanelType, SplitLayout};
use eframe::egui;

/// Manages the split layout integration
#[derive(Default)]
pub struct SplitLayoutManager {
    pub layout: SplitLayout,
}

impl SplitLayoutManager {
    /// Create with a server list panel
    pub fn with_server_list() -> Self {
        let layout = SplitLayout::default();
        // The default layout already has a server list
        Self { layout }
    }

    /// Create with terminal + server list layout
    pub fn with_terminal_layout(server_name: &str, session_id: &str) -> Self {
        let terminal_content =
            PanelContent::new(PanelType::Terminal, format!("Terminal - {}", server_name))
                .with_session(session_id.to_string());

        let server_list_content = PanelContent::new(PanelType::ServerList, "Servers");

        let root = LayoutNode::Horizontal {
            children: vec![
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: server_list_content,
                },
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: terminal_content,
                },
            ],
            ratios: vec![0.25, 0.75],
        };

        let mut layout = SplitLayout::default();
        layout.root = root;
        layout.update_panel_order();

        Self { layout }
    }

    /// Add a terminal panel
    pub fn add_terminal(&mut self, title: &str, session_id: &str) -> PanelId {
        let content =
            PanelContent::new(PanelType::Terminal, title).with_session(session_id.to_string());
        self.layout.add_panel(content)
    }

    /// Add a SFTP browser panel
    pub fn add_sftp(&mut self, title: &str, session_id: &str) -> PanelId {
        let content = PanelContent::new(PanelType::SftpBrowser, format!("SFTP - {}", title))
            .with_session(session_id.to_string());
        self.layout.add_panel(content)
    }

    /// Add a monitor panel
    pub fn add_monitor(&mut self, title: &str, session_id: &str) -> PanelId {
        let content = PanelContent::new(PanelType::Monitor, format!("Monitor - {}", title))
            .with_session(session_id.to_string());
        self.layout.add_panel(content)
    }

    /// Split a panel horizontally
    pub fn split_horizontal(
        &mut self,
        target_id: PanelId,
        panel_type: PanelType,
        title: &str,
    ) -> Option<PanelId> {
        let content = PanelContent::new(panel_type, title);
        self.layout.split_panel_horizontal(target_id, content)
    }

    /// Split a panel vertically
    pub fn split_vertical(
        &mut self,
        target_id: PanelId,
        panel_type: PanelType,
        title: &str,
    ) -> Option<PanelId> {
        let content = PanelContent::new(panel_type, title);
        self.layout.split_panel_vertical(target_id, content)
    }

    /// Close a panel
    pub fn close_panel(&mut self, id: PanelId) {
        self.layout.remove_panel(id);
    }

    /// Switch to panel by index (Alt+Number)
    pub fn switch_to_panel(&mut self, idx: usize) {
        if let Some(id) = self.layout.panel_by_index(idx) {
            self.layout.set_active_panel(id);
        }
    }

    /// Render the entire layout
    pub fn render<F>(&mut self, ui: &mut egui::Ui, mut render_panel: F)
    where
        F: FnMut(&mut egui::Ui, PanelId, &PanelContent, bool),
    {
        self.layout.render(ui, |ui, id, content, is_active| {
            render_panel(ui, id, content, is_active);
        });
    }

    /// Get active panel
    pub fn active_panel(&self) -> Option<PanelId> {
        self.layout.active_panel()
    }

    /// Set active panel
    pub fn set_active_panel(&mut self, id: PanelId) {
        self.layout.set_active_panel(id);
    }

    /// Get all panel IDs
    pub fn all_panels(&self) -> Vec<PanelId> {
        self.layout.all_panel_ids()
    }

    /// Apply a layout preset
    pub fn apply_preset(&mut self, preset: LayoutPreset) {
        match preset {
            LayoutPreset::Single => {
                // Reset to single server list panel
                let server_list = PanelContent::new(PanelType::ServerList, "Servers");
                self.layout.root = LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: server_list,
                };
            }
            LayoutPreset::TerminalSftp => {
                // Would need external data to create properly
            }
            LayoutPreset::Triple => {
                // Would need external data to create properly
            }
        }
        self.layout.update_panel_order();
    }

    /// Serialize current layout
    pub fn save_layout(&self) -> String {
        self.layout.serialize()
    }

    /// Deserialize and restore layout
    pub fn load_layout(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.layout.deserialize(json)
    }
}

/// Layout presets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutPreset {
    Single,
    TerminalSftp,
    Triple,
}

impl LayoutPreset {
    pub fn name(&self) -> &'static str {
        match self {
            LayoutPreset::Single => "Single",
            LayoutPreset::TerminalSftp => "Terminal + SFTP",
            LayoutPreset::Triple => "Triple Panel",
        }
    }
}
