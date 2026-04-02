#![allow(dead_code)]

//! Split Panel Layout System for EasySSH Windows UI
//!
//! Features:
//! - Horizontal and vertical splitting
//! - Multiple panel types: Terminal, SFTP, Monitor, ServerList
//! - Drag panels to edges for auto-split
//! - Panel title bars with type and server info
//! - Alt+Number quick switching
//! - Save/restore layout configuration
//! - Draggable splitters for resizing

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for panels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelId(pub u64);

impl PanelId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        PanelId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Types of panels that can be displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelType {
    Terminal,
    SftpBrowser,
    Monitor,
    ServerList,
}

impl PanelType {
    pub fn name(&self) -> &'static str {
        match self {
            PanelType::Terminal => "Terminal",
            PanelType::SftpBrowser => "SFTP",
            PanelType::Monitor => "Monitor",
            PanelType::ServerList => "Servers",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PanelType::Terminal => "🖥",
            PanelType::SftpBrowser => "📁",
            PanelType::Monitor => "📊",
            PanelType::ServerList => "📡",
        }
    }
}

/// Information about a panel's content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelContent {
    pub panel_type: PanelType,
    pub title: String,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub session_id: Option<String>,
}

impl PanelContent {
    pub fn new(panel_type: PanelType, title: impl Into<String>) -> Self {
        Self {
            panel_type,
            title: title.into(),
            server_id: None,
            server_name: None,
            session_id: None,
        }
    }

    pub fn with_server(mut self, id: String, name: String) -> Self {
        self.server_id = Some(id);
        self.server_name = Some(name);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Layout node in the tree structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    /// A leaf node containing a panel
    Leaf { id: PanelId, content: PanelContent },
    /// A horizontal split container
    Horizontal {
        children: Vec<LayoutNode>,
        ratios: Vec<f32>,
    },
    /// A vertical split container
    Vertical {
        children: Vec<LayoutNode>,
        ratios: Vec<f32>,
    },
}

impl Default for PanelContent {
    fn default() -> Self {
        Self {
            panel_type: PanelType::Terminal,
            title: String::new(),
            server_id: None,
            server_name: None,
            session_id: None,
        }
    }
}

impl LayoutNode {
    /// Get all panel IDs in this subtree
    pub fn collect_panel_ids(&self, ids: &mut Vec<PanelId>) {
        match self {
            LayoutNode::Leaf { id, .. } => ids.push(*id),
            LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } => {
                for child in children {
                    child.collect_panel_ids(ids);
                }
            }
        }
    }

    /// Find a panel by ID
    pub fn find_panel(&self, id: PanelId) -> Option<&PanelContent> {
        match self {
            LayoutNode::Leaf {
                id: leaf_id,
                content,
            } if *leaf_id == id => Some(content),
            LayoutNode::Leaf { .. } => None,
            LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } => {
                children.iter().find_map(|c| c.find_panel(id))
            }
        }
    }

    /// Find a panel by ID (mutable)
    pub fn find_panel_mut(&mut self, id: PanelId) -> Option<&mut PanelContent> {
        match self {
            LayoutNode::Leaf {
                id: leaf_id,
                content,
            } if *leaf_id == id => Some(content),
            LayoutNode::Leaf { .. } => None,
            LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } => {
                children.iter_mut().find_map(|c| c.find_panel_mut(id))
            }
        }
    }

    /// Remove a panel by ID, returns true if removed
    pub fn remove_panel(&mut self, id: PanelId) -> bool {
        match self {
            LayoutNode::Leaf { id: leaf_id, .. } => *leaf_id == id,
            LayoutNode::Horizontal { children, ratios }
            | LayoutNode::Vertical { children, ratios } => {
                let mut removed_idx = None;
                for (idx, child) in children.iter_mut().enumerate() {
                    if child.remove_panel(id) {
                        removed_idx = Some(idx);
                        break;
                    }
                }

                if let Some(idx) = removed_idx {
                    children.remove(idx);
                    if !ratios.is_empty() && idx < ratios.len() {
                        ratios.remove(idx);
                    }
                    // Redistribute ratios
                    Self::normalize_ratios(ratios, children.len());
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Split a panel horizontally
    pub fn split_horizontal(
        &mut self,
        target_id: PanelId,
        new_content: PanelContent,
    ) -> Option<PanelId> {
        if let LayoutNode::Leaf { id, content } = self {
            if *id == target_id {
                let new_id = PanelId::new();
                let old_content = std::mem::take(content);
                *self = LayoutNode::Horizontal {
                    children: vec![
                        LayoutNode::Leaf {
                            id: *id,
                            content: old_content,
                        },
                        LayoutNode::Leaf {
                            id: new_id,
                            content: new_content,
                        },
                    ],
                    ratios: vec![0.5, 0.5],
                };
                return Some(new_id);
            }
        }

        if let LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } =
            self
        {
            for child in children.iter_mut() {
                if let Some(id) = child.split_horizontal(target_id, new_content.clone()) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Split a panel vertically
    pub fn split_vertical(
        &mut self,
        target_id: PanelId,
        new_content: PanelContent,
    ) -> Option<PanelId> {
        if let LayoutNode::Leaf { id, content } = self {
            if *id == target_id {
                let new_id = PanelId::new();
                let old_content = std::mem::take(content);
                *self = LayoutNode::Vertical {
                    children: vec![
                        LayoutNode::Leaf {
                            id: *id,
                            content: old_content,
                        },
                        LayoutNode::Leaf {
                            id: new_id,
                            content: new_content,
                        },
                    ],
                    ratios: vec![0.5, 0.5],
                };
                return Some(new_id);
            }
        }

        if let LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } =
            self
        {
            for child in children.iter_mut() {
                if let Some(id) = child.split_vertical(target_id, new_content.clone()) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Normalize ratios to sum to 1.0
    fn normalize_ratios(ratios: &mut Vec<f32>, num_children: usize) {
        // Ensure correct count
        while ratios.len() < num_children {
            ratios.push(1.0 / num_children as f32);
        }
        if ratios.len() > num_children {
            ratios.truncate(num_children);
        }

        // Normalize to sum to 1.0
        let total: f32 = ratios.iter().sum();
        if total > 0.0 {
            for r in ratios.iter_mut() {
                *r /= total;
            }
        }
    }

    /// Simplify the tree by collapsing single-child containers
    pub fn simplify(&mut self) {
        match self {
            LayoutNode::Horizontal { children, .. } | LayoutNode::Vertical { children, .. } => {
                for child in children.iter_mut() {
                    child.simplify();
                }

                // If only one child, replace self with that child
                if children.len() == 1 {
                    let only_child = children.remove(0);
                    *self = only_child;
                }
            }
            _ => {}
        }
    }
}

/// Drop target for drag-and-drop splitting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropTarget {
    Left,
    Right,
    Top,
    Bottom,
    Center, // Replace/merge
}

/// State of the split layout system
#[derive(Debug)]
pub struct SplitLayout {
    pub root: LayoutNode,
    panels: HashMap<PanelId, PanelContent>,
    active_panel: Option<PanelId>,

    // Drag state
    dragging_panel: Option<PanelId>,
    drag_start_pos: Option<egui::Pos2>,

    // Resizing state
    resizing_splitter: Option<(Vec<usize>, bool)>, // (path indices, is_horizontal)
    resize_start_pos: f32,
    resize_start_ratios: Vec<f32>,

    // Panel order for Alt+Number switching
    panel_order: Vec<PanelId>,
}

impl Default for SplitLayout {
    fn default() -> Self {
        let mut panels = HashMap::new();
        let server_list_id = PanelId::new();
        panels.insert(
            server_list_id,
            PanelContent::new(PanelType::ServerList, "Servers"),
        );

        Self {
            root: LayoutNode::Leaf {
                id: server_list_id,
                content: panels[&server_list_id].clone(),
            },
            panels,
            active_panel: Some(server_list_id),
            dragging_panel: None,
            drag_start_pos: None,
            resizing_splitter: None,
            resize_start_pos: 0.0,
            resize_start_ratios: vec![],
            panel_order: vec![server_list_id],
        }
    }
}

impl SplitLayout {
    /// Create with initial terminal panel
    pub fn with_terminal(server_name: impl Into<String>, session_id: impl Into<String>) -> Self {
        let mut layout = Self::default();
        let server_name = server_name.into();
        let session_id = session_id.into();

        // Replace root with terminal
        let term_id = PanelId::new();
        let term_content =
            PanelContent::new(PanelType::Terminal, server_name.clone()).with_session(session_id);
        layout.panels.insert(term_id, term_content.clone());
        layout.root = LayoutNode::Leaf {
            id: term_id,
            content: term_content,
        };
        layout.active_panel = Some(term_id);
        layout.panel_order = vec![term_id];

        layout
    }

    /// Get the active panel ID
    pub fn active_panel(&self) -> Option<PanelId> {
        self.active_panel
    }

    /// Set active panel
    pub fn set_active_panel(&mut self, id: PanelId) {
        if self.panels.contains_key(&id) {
            self.active_panel = Some(id);
        }
    }

    /// Get panel by index for Alt+Number switching
    pub fn panel_by_index(&self, idx: usize) -> Option<PanelId> {
        self.panel_order.get(idx).copied()
    }

    /// Get panel content
    pub fn get_panel(&self, id: PanelId) -> Option<&PanelContent> {
        self.panels.get(&id)
    }

    /// Get mutable panel content
    pub fn get_panel_mut(&mut self, id: PanelId) -> Option<&mut PanelContent> {
        self.panels.get_mut(&id)
    }

    /// Add a new panel
    pub fn add_panel(&mut self, content: PanelContent) -> PanelId {
        let id = PanelId::new();
        self.panels.insert(id, content);
        self.panel_order.push(id);
        id
    }

    /// Split a panel horizontally (adds new panel to the right)
    pub fn split_panel_horizontal(
        &mut self,
        target_id: PanelId,
        new_content: PanelContent,
    ) -> Option<PanelId> {
        let new_id = self.add_panel(new_content);
        let result = self
            .root
            .split_horizontal(target_id, self.panels[&new_id].clone());
        if result.is_none() {
            // Rollback
            self.panels.remove(&new_id);
            self.panel_order.retain(|&id| id != new_id);
        }
        result
    }

    /// Split a panel vertically (adds new panel below)
    pub fn split_panel_vertical(
        &mut self,
        target_id: PanelId,
        new_content: PanelContent,
    ) -> Option<PanelId> {
        let new_id = self.add_panel(new_content);
        let result = self
            .root
            .split_vertical(target_id, self.panels[&new_id].clone());
        if result.is_none() {
            // Rollback
            self.panels.remove(&new_id);
            self.panel_order.retain(|&id| id != new_id);
        }
        result
    }

    /// Remove a panel
    pub fn remove_panel(&mut self, id: PanelId) {
        self.root.remove_panel(id);
        self.root.simplify();
        self.panels.remove(&id);
        self.panel_order.retain(|&pid| pid != id);

        if self.active_panel == Some(id) {
            self.active_panel = self.panel_order.first().copied();
        }
    }

    /// Collect all panel IDs
    pub fn all_panel_ids(&self) -> Vec<PanelId> {
        let mut ids = vec![];
        self.root.collect_panel_ids(&mut ids);
        ids
    }

    /// Update panel order based on layout traversal
    pub fn update_panel_order(&mut self) {
        self.panel_order = self.all_panel_ids();
    }

    /// Serialize layout to JSON
    pub fn serialize(&self) -> String {
        serde_json::to_string(&self.root).unwrap_or_default()
    }

    /// Deserialize layout from JSON
    pub fn deserialize(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.root = serde_json::from_str(json)?;
        self.update_panel_order();
        Ok(())
    }

    /// Start dragging a panel
    pub fn start_drag(&mut self, id: PanelId, pos: egui::Pos2) {
        self.dragging_panel = Some(id);
        self.drag_start_pos = Some(pos);
    }

    /// End dragging
    pub fn end_drag(&mut self) -> Option<(PanelId, DropTarget, Option<PanelId>)> {
        self.dragging_panel = None;
        self.drag_start_pos = None;
        None // Handled by UI during drag
    }

    /// Check if a panel is being dragged
    pub fn is_dragging(&self, id: PanelId) -> bool {
        self.dragging_panel == Some(id)
    }

    /// Start resizing a splitter
    pub fn start_resize(
        &mut self,
        path: Vec<usize>,
        is_horizontal: bool,
        pos: f32,
        ratios: Vec<f32>,
    ) {
        self.resizing_splitter = Some((path, is_horizontal));
        self.resize_start_pos = pos;
        self.resize_start_ratios = ratios;
    }

    /// End resizing
    pub fn end_resize(&mut self) {
        self.resizing_splitter = None;
    }

    /// Get resize state
    pub fn is_resizing(&self) -> Option<(Vec<usize>, bool)> {
        self.resizing_splitter.clone()
    }

    /// Render the layout with iterative approach to avoid stack overflow
    pub fn render<F>(&mut self, ui: &mut egui::Ui, mut render_panel: F)
    where
        F: FnMut(&mut egui::Ui, PanelId, &PanelContent, bool),
    {
        let available = ui.available_rect_before_wrap();
        let root = &mut self.root;
        let active = self.active_panel;

        // Use recursive approach but with explicit stack limits
        Self::render_node_recursive(ui, root, available, active, &mut render_panel, &[], 4.0, 0);
    }

    fn render_node_recursive<F>(
        ui: &mut egui::Ui,
        node: &mut LayoutNode,
        rect: egui::Rect,
        active: Option<PanelId>,
        render_panel: &mut F,
        path: &[usize],
        splitter_width: f32,
        depth: usize,
    ) where
        F: FnMut(&mut egui::Ui, PanelId, &PanelContent, bool),
    {
        // Safety limit to prevent stack overflow with extremely deep nesting
        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            return;
        }

        match node {
            LayoutNode::Leaf { id, content } => {
                let is_active = active == Some(*id);

                ui.allocate_ui_at_rect(rect, |ui| {
                    egui::Frame::group(ui.style())
                        .fill(if is_active {
                            egui::Color32::from_rgb(35, 38, 45)
                        } else {
                            egui::Color32::from_rgb(28, 31, 36)
                        })
                        .stroke(if is_active {
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(64, 156, 255))
                        } else {
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 55, 65))
                        })
                        .show(ui, |ui| {
                            ui.set_min_size(rect.size());
                            render_panel(ui, *id, content, is_active);
                        });
                });
            }
            LayoutNode::Horizontal { children, ratios } => {
                Self::render_split(
                    ui,
                    children,
                    ratios,
                    rect,
                    active,
                    render_panel,
                    path,
                    splitter_width,
                    true,
                    depth + 1,
                );
            }
            LayoutNode::Vertical { children, ratios } => {
                Self::render_split(
                    ui,
                    children,
                    ratios,
                    rect,
                    active,
                    render_panel,
                    path,
                    splitter_width,
                    false,
                    depth + 1,
                );
            }
        }
    }

    fn render_split<F>(
        ui: &mut egui::Ui,
        children: &mut Vec<LayoutNode>,
        ratios: &mut Vec<f32>,
        rect: egui::Rect,
        active: Option<PanelId>,
        render_panel: &mut F,
        path: &[usize],
        splitter_width: f32,
        is_horizontal: bool,
        depth: usize,
    ) where
        F: FnMut(&mut egui::Ui, PanelId, &PanelContent, bool),
    {
        let num_children = children.len();
        if num_children == 0 {
            return;
        }

        // Ensure ratios are normalized
        Self::normalize_ratios_internal(ratios, num_children);

        let total_size = if is_horizontal {
            rect.width() - (num_children as f32 - 1.0) * splitter_width
        } else {
            rect.height() - (num_children as f32 - 1.0) * splitter_width
        };

        let mut current_pos = if is_horizontal {
            rect.min.x
        } else {
            rect.min.y
        };

        // Use index-based iteration to satisfy borrow checker
        for idx in 0..num_children {
            let ratio = ratios
                .get(idx)
                .copied()
                .unwrap_or(1.0 / num_children as f32);
            let size = total_size * ratio.clamp(0.1, 0.9);

            let child_rect = if is_horizontal {
                egui::Rect::from_min_size(
                    egui::pos2(current_pos, rect.min.y),
                    egui::vec2(size, rect.height()),
                )
            } else {
                egui::Rect::from_min_size(
                    egui::pos2(rect.min.x, current_pos),
                    egui::vec2(rect.width(), size),
                )
            };

            // Recursively render each child
            if let Some(child) = children.get_mut(idx) {
                let mut new_path = path.to_vec();
                new_path.push(idx);
                Self::render_node_recursive(
                    ui,
                    child,
                    child_rect,
                    active,
                    render_panel,
                    &new_path,
                    splitter_width,
                    depth,
                );
            }

            current_pos += size;

            // Render splitter (except for last child)
            if idx < num_children - 1 {
                let splitter_pos = current_pos;
                let splitter_rect = if is_horizontal {
                    egui::Rect::from_min_size(
                        egui::pos2(splitter_pos, rect.min.y),
                        egui::vec2(splitter_width, rect.height()),
                    )
                } else {
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x, splitter_pos),
                        egui::vec2(rect.width(), splitter_width),
                    )
                };

                // Draw splitter line
                let line_pos = splitter_pos + splitter_width / 2.0;
                let is_hovered = ui.rect_contains_pointer(splitter_rect);

                if is_horizontal {
                    ui.painter().line_segment(
                        [
                            egui::pos2(line_pos, rect.min.y),
                            egui::pos2(line_pos, rect.max.y),
                        ],
                        egui::Stroke::new(
                            if is_hovered { 3.0 } else { 1.0 },
                            if is_hovered {
                                egui::Color32::from_rgb(64, 156, 255)
                            } else {
                                egui::Color32::from_rgb(50, 55, 65)
                            },
                        ),
                    );
                } else {
                    ui.painter().line_segment(
                        [
                            egui::pos2(rect.min.x, line_pos),
                            egui::pos2(rect.max.x, line_pos),
                        ],
                        egui::Stroke::new(
                            if is_hovered { 3.0 } else { 1.0 },
                            if is_hovered {
                                egui::Color32::from_rgb(64, 156, 255)
                            } else {
                                egui::Color32::from_rgb(50, 55, 65)
                            },
                        ),
                    );
                }

                // Handle resize interaction
                let resize_response =
                    ui.allocate_rect(splitter_rect.expand(4.0), egui::Sense::drag());

                if resize_response.dragged() {
                    let delta = if is_horizontal {
                        resize_response.drag_delta().x
                    } else {
                        resize_response.drag_delta().y
                    };

                    // Calculate new ratio with bounds checking
                    let new_size = (size + delta).clamp(50.0, total_size - 50.0);
                    let new_ratio = new_size / total_size;

                    // Update current and next ratio
                    if idx + 1 < ratios.len() {
                        let next_size = total_size * ratios[idx + 1];
                        let total_adjacent = size + next_size;
                        let new_next_size = total_adjacent - new_size;

                        if new_next_size >= 50.0 {
                            ratios[idx] = new_ratio;
                            ratios[idx + 1] = new_next_size / total_size;
                        }
                    }
                }

                current_pos += splitter_width;
            }
        }
    }

    fn normalize_ratios_internal(ratios: &mut Vec<f32>, num_children: usize) {
        if ratios.len() != num_children {
            // Fix ratio count
            if ratios.len() < num_children {
                let default_ratio = 1.0 / num_children as f32;
                while ratios.len() < num_children {
                    ratios.push(default_ratio);
                }
            } else {
                ratios.truncate(num_children);
            }
        }

        // Normalize to sum to 1.0
        let total: f32 = ratios.iter().sum();
        if total > 0.0 && (total - 1.0).abs() > 0.001 {
            for r in ratios.iter_mut() {
                *r /= total;
            }
        }
    }

    /// Get a suggested drop target based on mouse position within a rect
    pub fn drop_target_at_pos(
        rect: egui::Rect,
        pos: egui::Pos2,
        threshold: f32,
    ) -> Option<DropTarget> {
        let center = rect.center();
        let dx = pos.x - center.x;
        let dy = pos.y - center.y;
        let w = rect.width() / 2.0;
        let h = rect.height() / 2.0;

        // Normalize to -1 to 1
        let nx = dx / w;
        let ny = dy / h;

        // Check edges with threshold
        if nx.abs() > (1.0 - threshold) && ny.abs() < threshold {
            // Left or right edge
            if nx < 0.0 {
                Some(DropTarget::Left)
            } else {
                Some(DropTarget::Right)
            }
        } else if ny.abs() > (1.0 - threshold) && nx.abs() < threshold {
            // Top or bottom edge
            if ny < 0.0 {
                Some(DropTarget::Top)
            } else {
                Some(DropTarget::Bottom)
            }
        } else if nx.abs() < threshold && ny.abs() < threshold {
            // Center - replace/merge
            Some(DropTarget::Center)
        } else {
            None
        }
    }

    /// Render drop target indicators
    pub fn render_drop_indicators(ui: &mut egui::Ui, rect: egui::Rect, target: DropTarget) {
        let color = egui::Color32::from_rgba_premultiplied(64, 156, 255, 180);
        let stroke = egui::Stroke::new(3.0, color);

        match target {
            DropTarget::Left => {
                let indicator_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.min.x + rect.width() * 0.3, rect.max.y),
                );
                ui.painter().rect_stroke(indicator_rect, 4.0, stroke);
            }
            DropTarget::Right => {
                let indicator_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.max.x - rect.width() * 0.3, rect.min.y),
                    rect.max,
                );
                ui.painter().rect_stroke(indicator_rect, 4.0, stroke);
            }
            DropTarget::Top => {
                let indicator_rect = egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.3),
                );
                ui.painter().rect_stroke(indicator_rect, 4.0, stroke);
            }
            DropTarget::Bottom => {
                let indicator_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x, rect.max.y - rect.height() * 0.3),
                    rect.max,
                );
                ui.painter().rect_stroke(indicator_rect, 4.0, stroke);
            }
            DropTarget::Center => {
                ui.painter().rect_stroke(rect.shrink(8.0), 4.0, stroke);
            }
        }
    }
}

/// Layout presets for common configurations
pub struct LayoutPresets;

impl LayoutPresets {
    /// Single panel
    pub fn single(content: PanelContent) -> LayoutNode {
        LayoutNode::Leaf {
            id: PanelId::new(),
            content,
        }
    }

    /// Side-by-side terminal and file browser
    pub fn terminal_sftp(terminal: PanelContent, sftp: PanelContent) -> LayoutNode {
        LayoutNode::Horizontal {
            children: vec![
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: terminal,
                },
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: sftp,
                },
            ],
            ratios: vec![0.7, 0.3],
        }
    }

    /// Terminal with monitor below
    pub fn terminal_monitor(terminal: PanelContent, monitor: PanelContent) -> LayoutNode {
        LayoutNode::Vertical {
            children: vec![
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: terminal,
                },
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: monitor,
                },
            ],
            ratios: vec![0.75, 0.25],
        }
    }

    /// Three-way split: servers left, terminal center, sftp right
    pub fn triple_panel(
        servers: PanelContent,
        terminal: PanelContent,
        sftp: PanelContent,
    ) -> LayoutNode {
        LayoutNode::Horizontal {
            children: vec![
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: servers,
                },
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: terminal,
                },
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: sftp,
                },
            ],
            ratios: vec![0.2, 0.55, 0.25],
        }
    }

    /// Quad split: servers | terminal (top), monitor | sftp (bottom)
    pub fn quad_panel(
        servers: PanelContent,
        terminal: PanelContent,
        monitor: PanelContent,
        sftp: PanelContent,
    ) -> LayoutNode {
        LayoutNode::Horizontal {
            children: vec![
                LayoutNode::Leaf {
                    id: PanelId::new(),
                    content: servers,
                },
                LayoutNode::Vertical {
                    children: vec![
                        LayoutNode::Leaf {
                            id: PanelId::new(),
                            content: terminal,
                        },
                        LayoutNode::Horizontal {
                            children: vec![
                                LayoutNode::Leaf {
                                    id: PanelId::new(),
                                    content: monitor,
                                },
                                LayoutNode::Leaf {
                                    id: PanelId::new(),
                                    content: sftp,
                                },
                            ],
                            ratios: vec![0.5, 0.5],
                        },
                    ],
                    ratios: vec![0.7, 0.3],
                },
            ],
            ratios: vec![0.2, 0.8],
        }
    }
}
