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
    pub weight: f32, // 分屏比例权重
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
        children: Vec<String>, // panel IDs
        weights: Vec<f32>,
    },
}

/// 工作区布局
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub workspace_mode: WorkspaceMode,
    pub root_node: String, // root node ID
    #[serde(default)]
    pub nodes: HashMap<String, SplitNode>,
    #[serde(default)]
    pub panels: HashMap<String, Panel>,
    #[serde(default)]
    pub active_panel: Option<String>,
    #[serde(skip)]
    pub dirty: bool,
}

/// 工作区模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    Focus,     // 专注模式 - 单面板
    Split,     // 分屏模式
    Tabs,      // 标签页模式
    Dashboard, // 仪表盘模式
}

impl Default for WorkspaceMode {
    fn default() -> Self {
        WorkspaceMode::Focus
    }
}

impl Layout {
    /// 创建新的空白布局
    pub fn new(name: &str) -> Self {
        let id = Uuid::new_v4().to_string();
        let root_panel = Panel::new(PanelContent::Empty);
        let root_id = root_panel.id.clone();

        let mut panels = HashMap::new();
        panels.insert(root_id.clone(), root_panel);

        let mut nodes = HashMap::new();
        nodes.insert(root_id.clone(), SplitNode::Leaf { panel_id: root_id.clone() });

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

    /// 创建专注模式布局（单终端）
    pub fn focus_mode(name: &str, session_id: Option<String>) -> Self {
        let id = Uuid::new_v4().to_string();
        let content = PanelContent::Terminal { session_id, host_id: None };
        let root_panel = Panel::new(content).with_title(name);
        let root_id = root_panel.id.clone();

        let mut panels = HashMap::new();
        panels.insert(root_id.clone(), root_panel);

        let mut nodes = HashMap::new();
        nodes.insert(root_id.clone(), SplitNode::Leaf { panel_id: root_id.clone() });

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

    /// 创建分屏布局
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

    /// 分割面板
    pub fn split_panel(
        &mut self,
        panel_id: &str,
        direction: SplitDirection,
        new_content: PanelContent,
    ) -> Result<String, LiteError> {
        let new_panel = Panel::new(new_content);
        let new_id = new_panel.id.clone();

        // 找到包含该面板的节点
        if let Some(node_id) = self.find_node_containing_panel(panel_id) {
            if let Some(node) = self.nodes.get(&node_id).cloned() {
                match node {
                    SplitNode::Leaf { .. } => {
                        // 将叶子节点转换为分割节点
                        let split_id = Uuid::new_v4().to_string();
                        self.nodes.insert(
                            split_id.clone(),
                            SplitNode::Split {
                                direction,
                                children: vec![panel_id.to_string(), new_id.clone()],
                                weights: vec![0.5, 0.5],
                            },
                        );

                        // 替换原节点引用
                        if node_id == self.root_node {
                            self.root_node = split_id;
                        } else {
                            self.replace_node_in_parent(&node_id, &split_id);
                        }

                        // 移除旧节点
                        self.nodes.remove(&node_id);
                    }
                    SplitNode::Split { direction: existing_dir, mut children, mut weights } => {
                        if existing_dir == direction {
                            // 同方向，直接添加
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
                            // 不同方向，创建嵌套分割
                            let new_split_id = Uuid::new_v4().to_string();
                            self.nodes.insert(
                                new_split_id.clone(),
                                SplitNode::Split {
                                    direction,
                                    children: vec![panel_id.to_string(), new_id.clone()],
                                    weights: vec![0.5, 0.5],
                                },
                            );

                            // 替换原节点中的panel_id引用
                            let idx = children.iter().position(|id| id == panel_id)
                                .ok_or_else(|| LiteError::Layout("Panel not found in split".to_string()))?;
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

    /// 移除面板
    pub fn remove_panel(&mut self, panel_id: &str) -> Result<(), LiteError> {
        if self.panels.len() <= 1 {
            return Err(LiteError::Layout("Cannot remove the last panel".to_string()));
        }

        self.panels.remove(panel_id);

        // 更新或删除包含该面板的节点
        self.remove_panel_from_tree(panel_id)?;

        // 更新活动面板
        if self.active_panel.as_deref() == Some(panel_id) {
            self.active_panel = self.panels.keys().next().cloned();
        }

        self.dirty = true;
        Ok(())
    }

    /// 设置活动面板
    pub fn set_active_panel(&mut self, panel_id: &str) -> Result<(), LiteError> {
        if self.panels.contains_key(panel_id) {
            self.active_panel = Some(panel_id.to_string());
            self.dirty = true;
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Panel {} not found", panel_id)))
        }
    }

    /// 获取面板内容
    pub fn get_panel_content(&self, panel_id: &str) -> Option<&PanelContent> {
        self.panels.get(panel_id).map(|p| &p.content)
    }

    /// 更新面板内容
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

    /// 获取所有面板列表
    pub fn list_panels(&self) -> Vec<&Panel> {
        self.panels.values().collect()
    }

    /// 转换为JSON（用于持久化）
    pub fn to_json(&self) -> Result<String, LiteError> {
        serde_json::to_string(self).map_err(|e| LiteError::Json(e.to_string()))
    }

    /// 从JSON加载
    pub fn from_json(json: &str) -> Result<Self, LiteError> {
        let mut layout: Self = serde_json::from_str(json).map_err(|e| LiteError::Json(e.to_string()))?;
        layout.dirty = false;
        Ok(layout)
    }

    // 辅助方法：查找包含指定面板的节点
    fn find_node_containing_panel(&self, panel_id: &str) -> Option<String> {
        self.find_node_recursive(&self.root_node, panel_id)
    }

    fn find_node_recursive(&self, node_id: &str, panel_id: &str) -> Option<String> {
        if let Some(node) = self.nodes.get(node_id) {
            match node {
                SplitNode::Leaf { panel_id: id } if id == panel_id => Some(node_id.to_string()),
                SplitNode::Split { children, .. } => {
                    // First check if any child is the panel itself (direct panel reference)
                    for child_id in children {
                        if child_id == panel_id {
                            // This child is a direct panel reference, return this node
                            return Some(node_id.to_string());
                        }
                        // Otherwise recursively search
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

    // 辅助方法：在父节点中替换子节点
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

    // 辅助方法：从树中移除面板
    fn remove_panel_from_tree(&mut self, panel_id: &str) -> Result<(), LiteError> {
        let node_id = self.find_node_containing_panel(panel_id)
            .ok_or_else(|| LiteError::Layout(format!("Panel {} not in tree", panel_id)))?;

        if let Some(node) = self.nodes.get(&node_id).cloned() {
            match node {
                SplitNode::Leaf { .. } => {
                    if node_id == self.root_node {
                        // 根节点是叶子，不能删除
                        return Err(LiteError::Layout("Cannot remove root leaf".to_string()));
                    }
                    self.nodes.remove(&node_id);
                    // 需要在父节点中移除引用
                    self.remove_node_from_parent(&node_id);
                }
                SplitNode::Split { children, .. } => {
                    let new_children: Vec<String> = children.into_iter()
                        .filter(|id| id != panel_id)
                        .collect();

                    if new_children.len() == 1 {
                        // 只剩一个子节点，替换该节点为叶子
                        let remaining_id = new_children[0].clone();
                        // 创建叶子节点
                        self.nodes.insert(
                            remaining_id.clone(),
                            SplitNode::Leaf { panel_id: remaining_id.clone() },
                        );

                        if node_id == self.root_node {
                            self.root_node = remaining_id;
                        } else {
                            self.replace_node_in_parent(&node_id, &remaining_id);
                        }
                        self.nodes.remove(&node_id);
                    } else {
                        // 更新分割节点
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

    // 辅助方法：从父节点中移除子节点引用
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
        // 专注模式预设
        self.presets.insert(
            "focus".to_string(),
            Layout::new("专注模式"),
        );

        // 左右分屏预设
        self.presets.insert(
            "split_horizontal".to_string(),
            Layout::split_layout(
                "左右分屏",
                SplitDirection::Horizontal,
                PanelContent::Empty,
                PanelContent::ServerList { group_id: None },
            ),
        );

        // 上下分屏预设
        self.presets.insert(
            "split_vertical".to_string(),
            Layout::split_layout(
                "上下分屏",
                SplitDirection::Vertical,
                PanelContent::Empty,
                PanelContent::Snippets,
            ),
        );

        // 三栏布局预设
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

    /// 创建新布局
    pub fn create_layout(&mut self, name: &str, mode: WorkspaceMode) -> String {
        let mut layout = Layout::new(name);
        layout.workspace_mode = mode;
        let id = layout.id.clone();
        self.layouts.insert(id.clone(), layout);
        self.current_layout = Some(id.clone());
        id
    }

    /// 从预设创建布局
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

    /// 获取当前布局
    pub fn current_layout(&self) -> Option<&Layout> {
        self.current_layout.as_ref().and_then(|id| self.layouts.get(id))
    }

    /// 获取当前布局（可变）
    pub fn current_layout_mut(&mut self) -> Option<&mut Layout> {
        self.current_layout.as_ref().and_then(|id| self.layouts.get_mut(id))
    }

    /// 切换布局
    pub fn switch_layout(&mut self, layout_id: &str) -> Result<(), LiteError> {
        if self.layouts.contains_key(layout_id) {
            self.current_layout = Some(layout_id.to_string());
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Layout {} not found", layout_id)))
        }
    }

    /// 删除布局
    pub fn delete_layout(&mut self, layout_id: &str) -> Result<(), LiteError> {
        if self.layouts.len() <= 1 {
            return Err(LiteError::Layout("Cannot delete the only layout".to_string()));
        }

        self.layouts.remove(layout_id);

        if self.current_layout.as_deref() == Some(layout_id) {
            self.current_layout = self.layouts.keys().next().cloned();
        }

        Ok(())
    }

    /// 列出所有布局
    pub fn list_layouts(&self) -> Vec<&Layout> {
        self.layouts.values().collect()
    }

    /// 获取布局
    pub fn get_layout(&self, id: &str) -> Option<&Layout> {
        self.layouts.get(id)
    }

    /// 更新布局
    pub fn update_layout(&mut self, layout: Layout) -> Result<(), LiteError> {
        if self.layouts.contains_key(&layout.id) {
            self.layouts.insert(layout.id.clone(), layout);
            Ok(())
        } else {
            Err(LiteError::Layout(format!("Layout {} not found", layout.id)))
        }
    }

    /// 获取所有预设名称
    pub fn list_preset_names(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }

    /// 获取预设
    pub fn get_preset(&self, name: &str) -> Option<&Layout> {
        self.presets.get(name)
    }

    /// 保存布局到JSON
    pub fn export_layout(&self, layout_id: &str) -> Result<String, LiteError> {
        self.layouts.get(layout_id)
            .ok_or_else(|| LiteError::Layout(format!("Layout {} not found", layout_id)))?
            .to_json()
    }

    /// 从JSON导入布局
    pub fn import_layout(&mut self, json: &str) -> Result<String, LiteError> {
        let layout = Layout::from_json(json)?;
        let id = layout.id.clone();
        self.layouts.insert(id.clone(), layout);
        Ok(id)
    }

    /// 获取当前活动面板
    pub fn current_active_panel(&self) -> Option<(String, &Panel)> {
        self.current_layout().and_then(|layout| {
            layout.active_panel.as_ref().and_then(|panel_id| {
                layout.panels.get(panel_id).map(|p| (panel_id.clone(), p))
            })
        })
    }

    /// 在指定面板中打开终端
    pub fn open_terminal_in_panel(&mut self, panel_id: &str, session_id: Option<String>) -> Result<(), LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            layout.update_panel_content(panel_id, PanelContent::Terminal { session_id, host_id: None })
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    /// 分屏当前活动面板
    pub fn split_current_panel(
        &mut self,
        direction: SplitDirection,
        content: PanelContent,
    ) -> Result<String, LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            let active_id = layout.active_panel.clone()
                .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;
            layout.split_panel(&active_id, direction, content)
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }

    /// 关闭当前面板
    pub fn close_current_panel(&mut self) -> Result<(), LiteError> {
        if let Some(layout) = self.current_layout_mut() {
            let active_id = layout.active_panel.clone()
                .ok_or_else(|| LiteError::Layout("No active panel".to_string()))?;
            layout.remove_panel(&active_id)
        } else {
            Err(LiteError::Layout("No active layout".to_string()))
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
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

        let new_id = layout.split_panel(&root_id, SplitDirection::Horizontal, PanelContent::Snippets).unwrap();

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
            PanelContent::Terminal { session_id: Some("s1".to_string()), host_id: None },
            PanelContent::SftpBrowser { session_id: "s1".to_string(), path: "/home".to_string() },
        );

        let json = layout.to_json().unwrap();
        let restored = Layout::from_json(&json).unwrap();

        assert_eq!(restored.name, "序列化测试");
        assert_eq!(restored.panels.len(), 2);
    }

    #[test]
    fn test_panel_content_variants() {
        let terminal = PanelContent::Terminal { session_id: Some("s1".to_string()), host_id: Some("h1".to_string()) };
        let sftp = PanelContent::SftpBrowser { session_id: "s1".to_string(), path: "/home".to_string() };
        let snippets = PanelContent::Snippets;

        let t_json = serde_json::to_string(&terminal).unwrap();
        let s_json = serde_json::to_string(&sftp).unwrap();
        let n_json = serde_json::to_string(&snippets).unwrap();

        assert!(t_json.contains("terminal"));
        assert!(s_json.contains("sftp_browser"));
        assert!(n_json.contains("snippets"));
    }

    #[test]
    fn test_split_direction_serialization() {
        let h = SplitDirection::Horizontal;
        let v = SplitDirection::Vertical;

        assert_eq!(serde_json::to_string(&h).unwrap(), "\"horizontal\"");
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"vertical\"");
    }

    #[test]
    fn test_cannot_remove_last_panel() {
        let mut layout = Layout::new("测试");
        let panel_id = layout.panels.keys().next().cloned().unwrap();

        let result = layout.remove_panel(&panel_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_mode_serialization() {
        let modes = vec![
            WorkspaceMode::Focus,
            WorkspaceMode::Split,
            WorkspaceMode::Tabs,
            WorkspaceMode::Dashboard,
        ];

        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let restored: WorkspaceMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, restored);
        }
    }
}
