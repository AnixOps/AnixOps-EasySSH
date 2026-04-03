//! 分屏布局管理模块
//! 支持 Standard/Pro 版本的分屏功能

use crate::error::LiteError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 分屏方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    Horizontal, // 左右分屏
    Vertical,   // 上下分屏
}

impl SplitDirection {
    pub fn opposite(&self) -> Self {
        match self {
            SplitDirection::Horizontal => SplitDirection::Vertical,
            SplitDirection::Vertical => SplitDirection::Horizontal,
        }
    }
}

/// 面板内容类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PanelContent {
    Terminal {
        session_id: Option<String>,
        host_id: Option<String>,
    },
    SftpBrowser {
        session_id: String,
        path: String,
    },
    Monitoring {
        session_id: String,
        widget_type: String,
    },
    Snippets,
    ServerList {
        group_id: Option<String>,
    },
    Empty,
}

impl Default for PanelContent {
    fn default() -> Self {
        PanelContent::Empty
    }
}

/// 面板定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub content: PanelContent,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub weight: f32,
}

impl Panel {
    pub fn new(content: PanelContent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            title: None,
            weight: 1.0,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.1, 10.0);
        self
    }
}

/// 分屏节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "node_type")]
pub enum SplitNode {
    Leaf {
        panel_id: String,
    },
    Split {
        direction: SplitDirection,
        children: Vec<String>,
        weights: Vec<f32>,
    },
}

/// 工作区模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    Focus,
    Split,
    Tabs,
    Dashboard,
}

impl Default for WorkspaceMode {
    fn default() -> Self {
        WorkspaceMode::Focus
    }
}

/// 工作区布局
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub workspace_mode: WorkspaceMode,
    pub root_node: String,
    #[serde(default)]
    pub nodes: HashMap<String, SplitNode>,
    #[serde(default)]
    pub panels: HashMap<String, Panel>,
    #[serde(default)]
    pub active_panel: Option<String>,
    #[serde(skip)]
    pub dirty: bool,
}

impl Layout {
    pub fn new(name: &str) -> Self {
        let id = Uuid::new_v4().to_string();
        let root_panel = Panel::new(PanelContent::Empty);
        let root_id = root_panel.id.clone();

        let mut panels = HashMap::new();
        panels.insert(root_id.clone(), root_panel);

        let mut nodes = HashMap::new();
        nodes.insert(
            root_id.clone(),
            SplitNode::Leaf {
                panel_id: root_id.clone(),
            },
        );

        Self {
            id,
            name: name.to_string(),
            workspace_mode: WorkspaceMode::Focus,
            root_node: root_id.clone(),
            nodes,
            panels,
            active_panel: Some(root_id),
            dirty: true,
        }
    }

    pub fn focus_mode(name: &str, session_id: Option<String>) -> Self {
        let id = Uuid::new_v4().to_string();
        let content = PanelContent::Terminal {
            session_id,
            host_id: None,
        };
        let root_panel = Panel::new(content).with_title(name);
        let root_id = root_panel.id.clone();

        let mut panels = HashMap::new();
        panels.insert(root_id.clone(), root_panel);

        let mut nodes = HashMap::new();
        nodes.insert(
            root_id.clone(),
            SplitNode::Leaf {
                panel_id: root_id.clone(),
            },
        );

        Self {
            id,
            name: name.to_string(),
            workspace_mode: WorkspaceMode::Focus,
            root_node: root_id.clone(),
            nodes,
            panels,
            active_panel: Some(root_id),
            dirty: true,
        }
    }

    pub fn split_layout(
        name: &str,
        direction: SplitDirection,
        left_content: PanelContent,
        right_content: PanelContent,
    ) -> Self {
        let id = Uuid::new_v4().to_string();

        let left_panel = Panel::new(left_content);
        let right_panel = Panel::new(right_content);

        let left_id = left_panel.id.clone();
        let right_id = right_panel.id.clone();

        let mut panels = HashMap::new();
        panels.insert(left_id.clone(), left_panel);
        panels.insert(right_id.clone(), right_panel);

        let root_id = Uuid::new_v4().to_string();
        let mut nodes = HashMap::new();
        nodes.insert(
            root_id.clone(),
            SplitNode::Split {
                direction,
                children: vec![left_id.clone(), right_id.clone()],
                weights: vec![0.5, 0.5],
            },
        );

        Self {
            id,
            name: name.to_string(),
            workspace_mode: WorkspaceMode::Split,
            root_node: root_id,
            nodes,
            panels,
            active_panel: Some(left_id),
            dirty: true,
        }
    }

    pub fn split_panel(
        &mut self,
        panel_id: &str,
        direction: SplitDirection,
        new_content: PanelContent,
    ) -> Result<String, LiteError> {
        let new_panel = Panel::new(new_content);
        let new_id = new_panel.id.clone();

        if let Some(node_id) = self.find_node_containing_panel(panel_id) {
            if let Some(node) = self.nodes.get(&node_id).cloned() {
                match node {
                    SplitNode::Leaf { .. } => {
                        let split_id = Uuid::new_v4().to_string();
                        self.nodes.insert(
                            split_id.clone(),
                            SplitNode::Split {
                                direction,
                                children: vec![panel_id.to_string(), new_id.clone()],
                                weights: vec![0.5, 0.5],
                            },
                        );

                        if node_id == self.root_node {
                            self.root_node = split_id;
                        } else {
                            self.replace_node_in_parent(&node_id, &split_id);
                        }

                        self.nodes.remove(&node_id);
                    }
                    SplitNode::Split {
                        direction: existing_dir,
                        mut children,
                        mut weights,
                    } => {
                        if existing_dir == direction {
                            children.push(new_id.clone());
                            weights.push(0.5);
                            self.nodes.insert(
                                node_id,
                                SplitNode::Split {
                                    direction: existing_dir,
                                    children,
                                    weights,
                                },
                            );
                        } else {
                            let new_split_id = Uuid::new_v4().to_string();
                            self.nodes.insert(
                                new_split_id.clone(),
                                SplitNode::Split {
                                    direction,
                                    children: vec![panel_id.to_string(), new_id.clone()],
                                    weights: vec![0.5, 0.5],
                                },
                            );

                            let idx =
                                children
                                    .iter()
                                    .position(|id| id == panel_id)
                                    .ok_or_else(|| {
                                        LiteError::Layout("Panel not found in split".to_string())
                                    })?;
                            children[idx] = new_split_id;

                            self.nodes.insert(
                                node_id,
                                SplitNode::Split {
                                    direction: existing_dir,
                                    children,
                                    weights,
                                },
                            );
                        }
                    }
                }
            }
        } else {
            return Err(LiteError::Layout(format!("Panel {} not found", panel_id)));
        }

        self.panels.insert(new_id.clone(), new_panel);
        self.dirty = true;
        self.active_panel = Some(new_id.clone());

        Ok(new_id)
    }

    pub fn remove_panel(&mut self, panel_id: &str) -> Result<(), LiteError> {
        if self.panels.len() <= 1 {
            return Err(LiteError::Layout(
                "Cannot remove the last panel".to_string(),
            ));
        }

        self.panels.remove(panel_id);
        self.remove_panel_from_tree(panel_id)?;

        if self.active_panel.as_deref() == Some(panel_id) {
            self.active_panel = self.panels.keys().next().cloned();
        }

        self.dirty = true;
        Ok(())
    }

    pub fn set_active_panel(&mut self, panel_id: &str) -> Result<(), LiteError> {
        if self.panels.contains_key(panel_id) {
            self.active_panel = Some(panel_id.to_string());
            self.dirty = true;
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Panel {} not found", panel_id)))
        }
    }

    pub fn get_panel_content(&self, panel_id: &str) -> Option<&PanelContent> {
        self.panels.get(panel_id).map(|p| &p.content)
    }

    pub fn update_panel_content(
        &mut self,
        panel_id: &str,
        content: PanelContent,
    ) -> Result<(), LiteError> {
        if let Some(panel) = self.panels.get_mut(panel_id) {
            panel.content = content;
            self.dirty = true;
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Panel {} not found", panel_id)))
        }
    }

    pub fn list_panels(&self) -> Vec<&Panel> {
        self.panels.values().collect()
    }

    pub fn to_json(&self) -> Result<String, LiteError> {
        serde_json::to_string(self).map_err(|e| LiteError::Json(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Self, LiteError> {
        let mut layout: Self =
            serde_json::from_str(json).map_err(|e| LiteError::Json(e.to_string()))?;
        layout.dirty = false;
        Ok(layout)
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    fn find_node_containing_panel(&self, panel_id: &str) -> Option<String> {
        self.find_node_recursive(&self.root_node, panel_id)
    }

    fn find_node_recursive(&self, node_id: &str, panel_id: &str) -> Option<String> {
        if let Some(node) = self.nodes.get(node_id) {
            match node {
                SplitNode::Leaf { panel_id: id } if id == panel_id => Some(node_id.to_string()),
                SplitNode::Split { children, .. } => {
                    for child_id in children {
                        if child_id == panel_id {
                            return Some(node_id.to_string());
                        }
                        if let Some(found) = self.find_node_recursive(child_id, panel_id) {
                            return Some(found);
                        }
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn replace_node_in_parent(&mut self, old_id: &str, new_id: &str) {
        for node in self.nodes.values_mut() {
            if let SplitNode::Split { children, .. } = node {
                if let Some(idx) = children.iter().position(|id| id == old_id) {
                    children[idx] = new_id.to_string();
                    return;
                }
            }
        }
    }

    fn remove_panel_from_tree(&mut self, panel_id: &str) -> Result<(), LiteError> {
        let node_id = self
            .find_node_containing_panel(panel_id)
            .ok_or_else(|| LiteError::Layout(format!("Panel {} not in tree", panel_id)))?;

        if let Some(node) = self.nodes.get(&node_id).cloned() {
            match node {
                SplitNode::Leaf { .. } => {
                    if node_id == self.root_node {
                        return Err(LiteError::Layout("Cannot remove root leaf".to_string()));
                    }
                    self.nodes.remove(&node_id);
                    self.remove_node_from_parent(&node_id);
                }
                SplitNode::Split { children, .. } => {
                    let new_children: Vec<String> =
                        children.into_iter().filter(|id| id != panel_id).collect();

                    if new_children.len() == 1 {
                        let remaining_id = new_children[0].clone();
                        self.nodes.insert(
                            remaining_id.clone(),
                            SplitNode::Leaf {
                                panel_id: remaining_id.clone(),
                            },
                        );

                        if node_id == self.root_node {
                            self.root_node = remaining_id;
                        } else {
                            self.replace_node_in_parent(&node_id, &remaining_id);
                        }
                        self.nodes.remove(&node_id);
                    } else {
                        let weights = vec![1.0 / new_children.len() as f32; new_children.len()];
                        if let Some(SplitNode::Split { direction, .. }) = self.nodes.get(&node_id) {
                            self.nodes.insert(
                                node_id,
                                SplitNode::Split {
                                    direction: *direction,
                                    children: new_children,
                                    weights,
                                },
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn remove_node_from_parent(&mut self, node_id: &str) {
        for node in self.nodes.values_mut() {
            if let SplitNode::Split { children, .. } = node {
                children.retain(|id| id != node_id);
            }
        }
    }
}

/// 布局管理器
pub struct LayoutManager {
    layouts: HashMap<String, Layout>,
    current_layout: Option<String>,
    presets: HashMap<String, Layout>,
}

impl LayoutManager {
    pub fn new() -> Self {
        let mut manager = Self {
            layouts: HashMap::new(),
            current_layout: None,
            presets: HashMap::new(),
        };
        manager.init_presets();
        manager
    }

    fn init_presets(&mut self) {
        self.presets
            .insert("focus".to_string(), Layout::new("专注模式"));
        self.presets.insert(
            "split_horizontal".to_string(),
            Layout::split_layout(
                "左右分屏",
                SplitDirection::Horizontal,
                PanelContent::Empty,
                PanelContent::ServerList { group_id: None },
            ),
        );
        self.presets.insert(
            "split_vertical".to_string(),
            Layout::split_layout(
                "上下分屏",
                SplitDirection::Vertical,
                PanelContent::Empty,
                PanelContent::Snippets,
            ),
        );

        let mut triple_layout = Layout::split_layout(
            "三栏布局",
            SplitDirection::Horizontal,
            PanelContent::ServerList { group_id: None },
            PanelContent::Empty,
        );
        if let Ok(center_id) = triple_layout.split_panel(
            &triple_layout.active_panel.clone().unwrap_or_default(),
            SplitDirection::Horizontal,
            PanelContent::Snippets,
        ) {
            triple_layout.active_panel = Some(center_id);
        }
        self.presets.insert("triple".to_string(), triple_layout);
    }

    pub fn create_layout(&mut self, name: &str, mode: WorkspaceMode) -> String {
        let mut layout = Layout::new(name);
        layout.workspace_mode = mode;
        let id = layout.id.clone();
        self.layouts.insert(id.clone(), layout);
        self.current_layout = Some(id.clone());
        id
    }

    pub fn create_from_preset(&mut self, preset_name: &str) -> Option<String> {
        if let Some(preset) = self.presets.get(preset_name) {
            let mut layout = preset.clone();
            layout.id = Uuid::new_v4().to_string();
            layout.dirty = true;
            let id = layout.id.clone();
            self.layouts.insert(id.clone(), layout);
            self.current_layout = Some(id.clone());
            Some(id)
        } else {
            None
        }
    }

    pub fn current_layout(&self) -> Option<&Layout> {
        self.current_layout
            .as_ref()
            .and_then(|id| self.layouts.get(id))
    }

    pub fn current_layout_mut(&mut self) -> Option<&mut Layout> {
        self.current_layout
            .as_ref()
            .and_then(|id| self.layouts.get_mut(id))
    }

    pub fn switch_layout(&mut self, layout_id: &str) -> Result<(), LiteError> {
        if self.layouts.contains_key(layout_id) {
            self.current_layout = Some(layout_id.to_string());
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Layout {} not found", layout_id)))
        }
    }

    pub fn delete_layout(&mut self, layout_id: &str) -> Result<(), LiteError> {
        if self.layouts.len() <= 1 {
            return Err(LiteError::Layout(
                "Cannot delete the only layout".to_string(),
            ));
        }

        self.layouts.remove(layout_id);

        if self.current_layout.as_deref() == Some(layout_id) {
            self.current_layout = self.layouts.keys().next().cloned();
        }

        Ok(())
    }

    pub fn list_layouts(&self) -> Vec<&Layout> {
        self.layouts.values().collect()
    }

    pub fn get_layout(&self, id: &str) -> Option<&Layout> {
        self.layouts.get(id)
    }

    pub fn update_layout(&mut self, layout: Layout) -> Result<(), LiteError> {
        if self.layouts.contains_key(&layout.id) {
            self.layouts.insert(layout.id.clone(), layout);
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Layout {} not found", layout.id)))
        }
    }

    pub fn list_preset_names(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_preset(&self, name: &str) -> Option<&Layout> {
        self.presets.get(name)
    }

    pub fn export_layout(&self, layout_id: &str) -> Result<String, LiteError> {
        self.layouts
            .get(layout_id)
            .ok_or_else(|| LiteError::Layout(format!("Layout {} not found", layout_id)))?
            .to_json()
    }

    pub fn import_layout(&mut self, json: &str) -> Result<String, LiteError> {
        let layout = Layout::from_json(json)?;
        let id = layout.id.clone();
        self.layouts.insert(id.clone(), layout);
        Ok(id)
    }

    pub fn current_active_panel(&self) -> Option<(String, &Panel)> {
        self.current_layout().and_then(|layout| {
            layout
                .active_panel
                .as_ref()
                .and_then(|panel_id| layout.panels.get(panel_id).map(|p| (panel_id.clone(), p)))
        })
    }

    pub fn open_terminal_in_panel(
        &mut self,
        panel_id: &str,
        session_id: Option<String>,
    ) -> Result<(), LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            layout.update_panel_content(
                panel_id,
                PanelContent::Terminal {
                    session_id,
                    host_id: None,
                },
            )
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    pub fn split_current_panel(
        &mut self,
        direction: SplitDirection,
        content: PanelContent,
    ) -> Result<String, LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            let active_id = layout
                .active_panel
                .clone()
                .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;
            layout.split_panel(&active_id, direction, content)
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    pub fn split_panel(
        &mut self,
        panel_id: &str,
        direction: SplitDirection,
        content: PanelContent,
    ) -> Result<String, LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            layout.split_panel(panel_id, direction, content)
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    pub fn close_current_panel(&mut self) -> Result<(), LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            let active_id = layout
                .active_panel
                .clone()
                .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;
            layout.remove_panel(&active_id)
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    pub fn next_panel(&mut self) -> Result<(), LiteError> {
        let panels: Vec<String> = self
            .current_layout()
            .map(|l| l.panels.keys().cloned().collect())
            .unwrap_or_default();

        if panels.len() <= 1 {
            return Ok(());
        }

        let current = self
            .current_layout()
            .and_then(|l| l.active_panel.clone())
            .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;

        if let Some(idx) = panels.iter().position(|id| id == &current) {
            let next_idx = (idx + 1) % panels.len();
            if let Some(layout) = self.current_layout_mut() {
                layout.set_active_panel(&panels[next_idx])?;
            }
        }
        Ok(())
    }

    pub fn prev_panel(&mut self) -> Result<(), LiteError> {
        let panels: Vec<String> = self
            .current_layout()
            .map(|l| l.panels.keys().cloned().collect())
            .unwrap_or_default();

        if panels.len() <= 1 {
            return Ok(());
        }

        let current = self
            .current_layout()
            .and_then(|l| l.active_panel.clone())
            .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;

        if let Some(idx) = panels.iter().position(|id| id == &current) {
            let prev_idx = if idx == 0 { panels.len() - 1 } else { idx - 1 };
            if let Some(layout) = self.current_layout_mut() {
                layout.set_active_panel(&panels[prev_idx])?;
            }
        }
        Ok(())
    }

    pub fn export_all(&self) -> Result<String, LiteError> {
        let layouts: Vec<&Layout> = self.layouts.values().collect();
        serde_json::to_string(&layouts).map_err(|e| LiteError::Json(e.to_string()))
    }

    pub fn import_all(&mut self, json: &str) -> Result<Vec<String>, LiteError> {
        let layouts: Vec<Layout> =
            serde_json::from_str(json).map_err(|e| LiteError::Json(e.to_string()))?;

        let mut ids = Vec::new();
        for mut layout in layouts {
            layout.id = Uuid::new_v4().to_string();
            let id = layout.id.clone();
            self.layouts.insert(id.clone(), layout);
            ids.push(id);
        }
        Ok(ids)
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 增强功能 ============

/// 拖拽状态
#[derive(Debug, Clone, Default)]
pub struct DragDropState {
    pub is_dragging: bool,
    pub start_pos: (f32, f32),
    pub current_pos: (f32, f32),
    pub delta: (f32, f32),
}

impl DragDropState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, x: f32, y: f32) {
        self.is_dragging = true;
        self.start_pos = (x, y);
        self.current_pos = (x, y);
        self.delta = (0.0, 0.0);
    }

    pub fn update(&mut self, x: f32, y: f32) {
        self.delta = (x - self.current_pos.0, y - self.current_pos.1);
        self.current_pos = (x, y);
    }

    pub fn end(&mut self) {
        *self = Self::default();
    }
}

/// 矩形区域
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }
}

/// 分割条样式
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitHandleStyle {
    pub size: f32,
    pub hover_size: f32,
    pub color: [u8; 4],
    pub hover_color: [u8; 4],
}

impl Default for SplitHandleStyle {
    fn default() -> Self {
        Self {
            size: 4.0,
            hover_size: 6.0,
            color: [100, 100, 100, 255],
            hover_color: [66, 133, 244, 255],
        }
    }
}

/// 布局主题
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutTheme {
    pub background_color: [u8; 4],
    pub panel_background: [u8; 4],
    pub split_handle: SplitHandleStyle,
}

impl Default for LayoutTheme {
    fn default() -> Self {
        Self {
            background_color: [30, 30, 30, 255],
            panel_background: [40, 40, 40, 255],
            split_handle: SplitHandleStyle::default(),
        }
    }
}

/// 布局配置
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub theme: LayoutTheme,
    pub min_panel_size: f32,
    pub default_split_ratio: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            theme: LayoutTheme::default(),
            min_panel_size: 100.0,
            default_split_ratio: 0.5,
        }
    }
}

/// 布局工具函数
pub mod layout_utils {
    use super::*;

    pub fn calculate_split_rects(
        parent: &Rect,
        direction: SplitDirection,
        ratios: &[f32],
    ) -> Vec<Rect> {
        let mut rects = Vec::with_capacity(ratios.len());
        let mut current_pos = 0.0;

        for ratio in ratios {
            match direction {
                SplitDirection::Horizontal => {
                    let width = parent.width * ratio;
                    rects.push(Rect::new(
                        parent.x + current_pos,
                        parent.y,
                        width,
                        parent.height,
                    ));
                    current_pos += width;
                }
                SplitDirection::Vertical => {
                    let height = parent.height * ratio;
                    rects.push(Rect::new(
                        parent.x,
                        parent.y + current_pos,
                        parent.width,
                        height,
                    ));
                    current_pos += height;
                }
            }
        }
        rects
    }

    pub fn normalize_ratios(ratios: &mut [f32]) {
        let sum: f32 = ratios.iter().sum();
        if sum > 0.0 {
            for ratio in ratios.iter_mut() {
                *ratio /= sum;
            }
        }
    }
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_new() {
        let layout = Layout::new("测试布局");
        assert_eq!(layout.name, "测试布局");
        assert_eq!(layout.workspace_mode, WorkspaceMode::Focus);
        assert_eq!(layout.panels.len(), 1);
    }

    #[test]
    fn test_split_layout() {
        let layout = Layout::split_layout(
            "分屏测试",
            SplitDirection::Horizontal,
            PanelContent::Empty,
            PanelContent::Snippets,
        );
        assert_eq!(layout.panels.len(), 2);
        assert_eq!(layout.workspace_mode, WorkspaceMode::Split);
    }

    #[test]
    fn test_split_panel() {
        let mut layout = Layout::new("测试");
        let root_id = layout.root_node.clone();

        let new_id = layout
            .split_panel(&root_id, SplitDirection::Horizontal, PanelContent::Snippets)
            .unwrap();

        assert_eq!(layout.panels.len(), 2);
        assert!(layout.panels.contains_key(&new_id));
        assert_eq!(layout.active_panel, Some(new_id));
    }

    #[test]
    fn test_remove_panel() {
        let mut layout = Layout::split_layout(
            "测试",
            SplitDirection::Horizontal,
            PanelContent::Empty,
            PanelContent::Snippets,
        );

        let panels: Vec<String> = layout.panels.keys().cloned().collect();
        assert_eq!(panels.len(), 2);

        layout.remove_panel(&panels[0]).unwrap();

        assert_eq!(layout.panels.len(), 1);
    }

    #[test]
    fn test_layout_manager_presets() {
        let manager = LayoutManager::new();
        let presets = manager.list_preset_names();
        assert!(presets.contains(&"focus"));
        assert!(presets.contains(&"split_horizontal"));
        assert!(presets.contains(&"split_vertical"));
    }

    #[test]
    fn test_create_from_preset() {
        let mut manager = LayoutManager::new();
        let id = manager.create_from_preset("focus").unwrap();
        assert!(manager.get_layout(&id).is_some());
        assert_eq!(manager.current_layout, Some(id));
    }

    #[test]
    fn test_layout_serialization() {
        let layout = Layout::split_layout(
            "序列化测试",
            SplitDirection::Vertical,
            PanelContent::Terminal {
                session_id: Some("s1".to_string()),
                host_id: None,
            },
            PanelContent::SftpBrowser {
                session_id: "s1".to_string(),
                path: "/home".to_string(),
            },
        );

        let json = layout.to_json().unwrap();
        let restored = Layout::from_json(&json).unwrap();

        assert_eq!(restored.name, "序列化测试");
        assert_eq!(restored.panels.len(), 2);
    }

    #[test]
    fn test_cannot_remove_last_panel() {
        let mut layout = Layout::new("测试");
        let panel_id = layout.panels.keys().next().cloned().unwrap();

        let result = layout.remove_panel(&panel_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_drag_drop_state() {
        let mut state = DragDropState::new();
        assert!(!state.is_dragging);
        state.start(100.0, 100.0);
        assert!(state.is_dragging);
        state.update(150.0, 150.0);
        assert_eq!(state.delta.0, 50.0);
        state.end();
        assert!(!state.is_dragging);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains((50.0, 50.0)));
        assert!(!rect.contains((150.0, 50.0)));
    }

    #[test]
    fn test_layout_utils() {
        let parent = Rect::new(0.0, 0.0, 100.0, 100.0);
        let ratios = vec![0.3, 0.7];
        let rects =
            layout_utils::calculate_split_rects(&parent, SplitDirection::Horizontal, &ratios);
        assert_eq!(rects.len(), 2);
        // Use approximate comparison for floating point
        assert!((rects[0].width - 30.0).abs() < 0.001);
        assert!((rects[1].width - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_next_prev_panel() {
        let mut manager = LayoutManager::new();
        // Create a layout first
        manager.create_layout("Test Layout", WorkspaceMode::Split);

        let panels: Vec<String> = manager
            .current_layout()
            .map(|l| l.panels.keys().cloned().collect())
            .unwrap();
        let root = &panels[0];

        // Split the root panel to create two panels
        manager
            .split_panel(
                root,
                SplitDirection::Horizontal,
                PanelContent::Empty,
            )
            .unwrap();

        let initial = manager.current_layout().unwrap().active_panel.clone();
        manager.next_panel().unwrap();
        let after_next = manager.current_layout().unwrap().active_panel.clone();
        assert_ne!(initial, after_next);
    }
}
