//! 多标签页终端管理
//! 支持标签页创建、切换、拖拽、分组等功能

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::error::LiteError;

/// 标签页信息
#[derive(Debug, Clone)]
pub struct TabInfo {
    /// 标签页ID
    pub id: String,
    /// 标签页标题
    pub title: String,
    /// 标签页状态
    pub state: TabState,
    /// 终端ID（如果已连接）
    pub terminal_id: Option<String>,
    /// 服务器ID（如果是SSH会话）
    pub server_id: Option<String>,
    /// 会话类型
    pub session_type: SessionType,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后活动时间
    pub last_active_at: SystemTime,
    /// 索引位置
    pub index: usize,
    /// 所属分组
    pub group: Option<String>,
    /// 图标（可选）
    pub icon: Option<String>,
    /// 是否固定
    pub is_pinned: bool,
}

impl TabInfo {
    /// 创建新标签页
    pub fn new(title: &str, session_type: SessionType) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            state: TabState::Initializing,
            terminal_id: None,
            server_id: None,
            session_type,
            created_at: now,
            last_active_at: now,
            index: 0,
            group: None,
            icon: None,
            is_pinned: false,
        }
    }

    /// 标记为活跃
    pub fn touch(&mut self) {
        self.last_active_at = SystemTime::now();
    }

    /// 获取空闲时间
    pub fn idle_duration(&self) -> std::time::Duration {
        self.last_active_at.elapsed().unwrap_or_default()
    }

    /// 是否空闲超时
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.idle_duration().as_secs() > timeout_secs
    }
}

/// 标签页状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    /// 初始化中
    Initializing,
    /// 连接中
    Connecting,
    /// 已连接/活跃
    Active,
    /// 断开连接
    Disconnected,
    /// 重新连接中
    Reconnecting,
    /// 已关闭
    Closed,
    /// 出错
    Error,
}

impl TabState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TabState::Initializing => "initializing",
            TabState::Connecting => "connecting",
            TabState::Active => "active",
            TabState::Disconnected => "disconnected",
            TabState::Reconnecting => "reconnecting",
            TabState::Closed => "closed",
            TabState::Error => "error",
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, TabState::Active | TabState::Connecting | TabState::Reconnecting)
    }

    pub fn can_close(&self) -> bool {
        !matches!(self, TabState::Closed)
    }
}

/// 会话类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    /// 本地shell
    LocalShell,
    /// SSH会话
    Ssh,
    /// 串口连接
    Serial,
    /// Telnet
    Telnet,
    /// Docker容器
    Docker,
    /// Kubernetes Pod
    Kubernetes,
    /// WSL
    Wsl,
    /// 远程桌面（RDP/VNC）
    RemoteDesktop,
}

impl SessionType {
    pub fn default_title(&self) -> &'static str {
        match self {
            SessionType::LocalShell => "Local",
            SessionType::Ssh => "SSH",
            SessionType::Serial => "Serial",
            SessionType::Telnet => "Telnet",
            SessionType::Docker => "Docker",
            SessionType::Kubernetes => "K8s",
            SessionType::Wsl => "WSL",
            SessionType::RemoteDesktop => "RDP",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SessionType::LocalShell => "terminal",
            SessionType::Ssh => "ssh",
            SessionType::Serial => "usb",
            SessionType::Telnet => "network",
            SessionType::Docker => "docker",
            SessionType::Kubernetes => "kubernetes",
            SessionType::Wsl => "linux",
            SessionType::RemoteDesktop => "monitor",
        }
    }
}

/// 标签页管理器
pub struct TabManager {
    /// 所有标签页
    tabs: Arc<RwLock<HashMap<String, TabInfo>>>,
    /// 标签页顺序
    order: Arc<RwLock<Vec<String>>>,
    /// 当前活跃标签页
    active: Arc<RwLock<Option<String>>>,
    /// 标签页分组
    groups: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// 事件发送器
    event_tx: mpsc::UnboundedSender<TabEvent>,
    /// 事件接收器（用于外部监听）
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<TabEvent>>>,
    /// 标签页ID生成器
    next_index: Arc<RwLock<usize>>,
    /// 空闲超时时间（秒）
    idle_timeout: u64,
}

/// 标签页事件
#[derive(Debug, Clone)]
pub enum TabEvent {
    /// 标签页创建
    Created { tab_id: String },
    /// 标签页关闭
    Closed { tab_id: String },
    /// 标签页激活
    Activated { tab_id: String },
    /// 标签页标题变更
    TitleChanged { tab_id: String, title: String },
    /// 标签页状态变更
    StateChanged { tab_id: String, state: TabState },
    /// 标签页顺序变更
    OrderChanged { order: Vec<String> },
    /// 标签页分组变更
    GroupChanged { tab_id: String, group: Option<String> },
    /// 所有标签页关闭
    AllClosed,
}

impl TabManager {
    /// 创建新标签页管理器
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            tabs: Arc::new(RwLock::new(HashMap::new())),
            order: Arc::new(RwLock::new(Vec::new())),
            active: Arc::new(RwLock::new(None)),
            groups: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            next_index: Arc::new(RwLock::new(0)),
            idle_timeout: 3600, // 1小时默认空闲超时
        }
    }

    /// 创建新标签页
    pub async fn create_tab(
        &self,
        title: &str,
        session_type: SessionType,
        group: Option<&str>,
    ) -> Result<String, LiteError> {
        let mut tab = TabInfo::new(title, session_type);

        // 分配索引
        {
            let mut next = self.next_index.write().await;
            tab.index = *next;
            *next += 1;
        }

        // 设置分组
        if let Some(g) = group {
            tab.group = Some(g.to_string());
            let mut groups = self.groups.write().await;
            groups.entry(g.to_string())
                .or_insert_with(Vec::new)
                .push(tab.id.clone());
        }

        let tab_id = tab.id.clone();

        // 添加到标签页集合
        {
            let mut tabs = self.tabs.write().await;
            tabs.insert(tab_id.clone(), tab);
        }

        // 添加到顺序列表
        {
            let mut order = self.order.write().await;
            order.push(tab_id.clone());
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::Created { tab_id: tab_id.clone() });

        // 设置为活跃
        self.activate_tab(&tab_id).await?;

        log::info!("Created tab: {} (type: {:?})", tab_id, session_type);

        Ok(tab_id)
    }

    /// 关闭标签页
    pub async fn close_tab(&self, tab_id: &str) -> Result<(), LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        if !tab.state.can_close() {
            return Err(LiteError::Terminal(format!("Tab {} cannot be closed (state: {:?})", tab_id, tab.state)));
        }

        // 从分组中移除
        if let Some(ref group) = tab.group {
            let mut groups = self.groups.write().await;
            if let Some(group_tabs) = groups.get_mut(group) {
                group_tabs.retain(|id| id != tab_id);
            }
        }

        // 更新状态为已关闭
        if let Some(tab) = tabs.get_mut(tab_id) {
            tab.state = TabState::Closed;
        }

        drop(tabs);

        // 从顺序列表移除
        {
            let mut order = self.order.write().await;
            order.retain(|id| id != tab_id);
        }

        // 更新活跃标签页
        {
            let mut active = self.active.write().await;
            if active.as_ref() == Some(&tab_id.to_string()) {
                // 切换到上一个标签页
                let order = self.order.read().await;
                *active = order.last().cloned();

                if let Some(ref new_active) = *active {
                    let _ = self.event_tx.send(TabEvent::Activated {
                        tab_id: new_active.clone()
                    });
                }
            }
        }

        // 最终从集合中移除
        {
            let mut tabs = self.tabs.write().await;
            tabs.remove(tab_id);
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::Closed { tab_id: tab_id.to_string() });

        log::info!("Closed tab: {}", tab_id);

        Ok(())
    }

    /// 激活标签页
    pub async fn activate_tab(&self, tab_id: &str) -> Result<(), LiteError> {
        let tabs = self.tabs.read().await;

        if !tabs.contains_key(tab_id) {
            return Err(LiteError::Terminal(format!("Tab {} not found", tab_id)));
        }

        drop(tabs);

        // 更新上一个标签页的最后活动时间
        {
            let active = self.active.read().await;
            if let Some(ref prev_id) = *active {
                let mut tabs = self.tabs.write().await;
                if let Some(tab) = tabs.get_mut(prev_id) {
                    tab.touch();
                }
            }
        }

        // 设置新的活跃标签页
        {
            let mut active = self.active.write().await;
            *active = Some(tab_id.to_string());
        }

        // 更新活跃时间
        {
            let mut tabs = self.tabs.write().await;
            if let Some(tab) = tabs.get_mut(tab_id) {
                tab.touch();
            }
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::Activated { tab_id: tab_id.to_string() });

        Ok(())
    }

    /// 获取当前活跃标签页ID
    pub async fn get_active(&self) -> Option<String> {
        self.active.read().await.clone()
    }

    /// 获取标签页信息
    pub async fn get_tab(&self, tab_id: &str) -> Option<TabInfo> {
        self.tabs.read().await.get(tab_id).cloned()
    }

    /// 更新标签页标题
    pub async fn set_title(&self, tab_id: &str, title: &str) -> Result<(), LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get_mut(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        tab.title = title.to_string();

        // 发送事件
        let _ = self.event_tx.send(TabEvent::TitleChanged {
            tab_id: tab_id.to_string(),
            title: title.to_string(),
        });

        Ok(())
    }

    /// 更新标签页状态
    pub async fn set_state(&self, tab_id: &str, state: TabState) -> Result<(), LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get_mut(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        tab.state = state;

        // 发送事件
        let _ = self.event_tx.send(TabEvent::StateChanged {
            tab_id: tab_id.to_string(),
            state,
        });

        Ok(())
    }

    /// 关联终端到标签页
    pub async fn attach_terminal(&self, tab_id: &str, terminal_id: &str) -> Result<(), LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get_mut(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        tab.terminal_id = Some(terminal_id.to_string());
        tab.state = TabState::Active;

        Ok(())
    }

    /// 获取标签页的终端ID
    pub async fn get_terminal_id(&self, tab_id: &str) -> Option<String> {
        let tabs = self.tabs.read().await;
        tabs.get(tab_id).and_then(|t| t.terminal_id.clone())
    }

    /// 列出所有标签页
    pub async fn list_tabs(&self) -> Vec<TabInfo> {
        let order = self.order.read().await;
        let tabs = self.tabs.read().await;

        order.iter()
            .filter_map(|id| tabs.get(id).cloned())
            .collect()
    }

    /// 获取所有标签页ID（按顺序）
    pub async fn get_order(&self) -> Vec<String> {
        self.order.read().await.clone()
    }

    /// 重新排序标签页
    pub async fn reorder(&self, new_order: Vec<String>) -> Result<(), LiteError> {
        // 验证所有ID存在
        let tabs = self.tabs.read().await;
        for id in &new_order {
            if !tabs.contains_key(id) {
                return Err(LiteError::Terminal(format!("Tab {} not found", id)));
            }
        }

        // 更新顺序
        {
            let mut order = self.order.write().await;
            *order = new_order.clone();
        }

        // 更新索引
        {
            let mut tabs = self.tabs.write().await;
            for (index, id) in new_order.iter().enumerate() {
                if let Some(tab) = tabs.get_mut(id) {
                    tab.index = index;
                }
            }
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::OrderChanged { order: new_order });

        Ok(())
    }

    /// 移动标签页位置
    pub async fn move_tab(&self, tab_id: &str, new_index: usize) -> Result<(), LiteError> {
        let mut order = self.order.write().await;

        // 找到当前位置
        let current_index = order.iter()
            .position(|id| id == tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        // 移除并插入到新位置
        let id = order.remove(current_index);
        let new_index = new_index.min(order.len());
        order.insert(new_index, id);

        drop(order);

        // 发送重新排序事件
        let new_order = self.order.read().await.clone();
        let _ = self.event_tx.send(TabEvent::OrderChanged { order: new_order });

        Ok(())
    }

    /// 设置标签页分组
    pub async fn set_group(&self, tab_id: &str, group: Option<&str>) -> Result<(), LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get_mut(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        // 从旧分组移除
        if let Some(ref old_group) = tab.group {
            let mut groups = self.groups.write().await;
            if let Some(group_tabs) = groups.get_mut(old_group) {
                group_tabs.retain(|id| id != tab_id);
            }
        }

        // 设置新分组
        tab.group = group.map(|g| g.to_string());

        // 添加到新分组
        if let Some(ref new_group) = tab.group {
            let mut groups = self.groups.write().await;
            groups.entry(new_group.clone())
                .or_insert_with(Vec::new)
                .push(tab_id.to_string());
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::GroupChanged {
            tab_id: tab_id.to_string(),
            group: tab.group.clone(),
        });

        Ok(())
    }

    /// 获取分组列表
    pub async fn list_groups(&self) -> Vec<String> {
        let groups = self.groups.read().await;
        groups.keys().cloned().collect()
    }

    /// 获取分组中的标签页
    pub async fn get_group_tabs(&self, group: &str) -> Vec<String> {
        let groups = self.groups.read().await;
        groups.get(group).cloned().unwrap_or_default()
    }

    /// 固定/取消固定标签页
    pub async fn toggle_pin(&self, tab_id: &str) -> Result<bool, LiteError> {
        let mut tabs = self.tabs.write().await;

        let tab = tabs.get_mut(tab_id)
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        tab.is_pinned = !tab.is_pinned;
        Ok(tab.is_pinned)
    }

    /// 关闭所有标签页
    pub async fn close_all(&self) -> Result<(), LiteError> {
        let tab_ids: Vec<String> = {
            let order = self.order.read().await;
            order.clone()
        };

        for id in tab_ids {
            let _ = self.close_tab(&id).await;
        }

        // 重置计数器
        {
            let mut next = self.next_index.write().await;
            *next = 0;
        }

        // 发送事件
        let _ = self.event_tx.send(TabEvent::AllClosed);

        Ok(())
    }

    /// 关闭非固定标签页
    pub async fn close_unpinned(&self) -> Result<usize, LiteError> {
        let unpinned: Vec<String> = {
            let tabs = self.tabs.read().await;
            tabs.values()
                .filter(|t| !t.is_pinned)
                .map(|t| t.id.clone())
                .collect()
        };

        let count = unpinned.len();

        for id in unpinned {
            let _ = self.close_tab(&id).await;
        }

        Ok(count)
    }

    /// 获取活跃标签页数量
    pub async fn count(&self) -> usize {
        self.tabs.read().await.len()
    }

    /// 获取固定标签页数量
    pub async fn pinned_count(&self) -> usize {
        let tabs = self.tabs.read().await;
        tabs.values().filter(|t| t.is_pinned).count()
    }

    /// 清理空闲标签页
    pub async fn cleanup_idle(&self) -> Result<usize, LiteError> {
        let idle_tabs: Vec<String> = {
            let tabs = self.tabs.read().await;
            tabs.values()
                .filter(|t| !t.is_pinned && t.is_idle(self.idle_timeout))
                .map(|t| t.id.clone())
                .collect()
        };

        let count = idle_tabs.len();

        for id in idle_tabs {
            let _ = self.close_tab(&id).await;
        }

        Ok(count)
    }

    /// 克隆标签页（在新标签页中打开相同的会话）
    pub async fn duplicate_tab(&self, tab_id: &str) -> Result<String, LiteError> {
        let source = self.get_tab(tab_id).await
            .ok_or_else(|| LiteError::Terminal(format!("Tab {} not found", tab_id)))?;

        let new_tab_id = self.create_tab(
            &format!("{} (Copy)", source.title),
            source.session_type,
            source.group.as_deref(),
        ).await?;

        // 复制server_id
        if let Some(server_id) = source.server_id {
            let mut tabs = self.tabs.write().await;
            if let Some(tab) = tabs.get_mut(&new_tab_id) {
                tab.server_id = Some(server_id);
            }
        }

        Ok(new_tab_id)
    }

    /// 获取下一个可用标签页标题（用于自动命名）
    pub async fn next_tab_title(&self, base: &str) -> String {
        let tabs = self.tabs.read().await;
        let count = tabs.values()
            .filter(|t| t.title.starts_with(base))
            .count();

        if count == 0 {
            base.to_string()
        } else {
            format!("{} ({})", base, count + 1)
        }
    }

    /// 订阅标签页事件
    pub async fn subscribe_events(&self) -> mpsc::UnboundedReceiver<TabEvent> {
        let (tx, rx) = mpsc::unbounded_channel();

        // 克隆当前事件接收器并转发
        let event_rx = self.event_rx.clone();
        tokio::spawn(async move {
            let mut rx = event_rx.write().await;
            while let Some(event) = rx.recv().await {
                let _ = tx.send(event.clone());
            }
        });

        rx
    }

    /// 获取标签页统计
    pub async fn get_stats(&self) -> TabStats {
        let tabs = self.tabs.read().await;

        let total = tabs.len();
        let active = tabs.values().filter(|t| t.state == TabState::Active).count();
        let idle = tabs.values().filter(|t| t.is_idle(self.idle_timeout)).count();
        let pinned = tabs.values().filter(|t| t.is_pinned).count();

        let groups = self.groups.read().await;

        TabStats {
            total,
            active,
            idle,
            pinned,
            groups: groups.len(),
        }
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 标签页统计
#[derive(Debug, Clone)]
pub struct TabStats {
    pub total: usize,
    pub active: usize,
    pub idle: usize,
    pub pinned: usize,
    pub groups: usize,
}

/// 标签页拖拽状态
#[derive(Debug, Clone)]
pub struct TabDragState {
    pub dragged_tab_id: String,
    pub source_index: usize,
    pub target_index: Option<usize>,
    pub is_over_valid_drop: bool,
}

/// 分屏布局管理器
pub struct SplitLayout {
    /// 标签页ID到分屏容器的映射
    tab_containers: HashMap<String, SplitContainer>,
}

/// 分屏容器
#[derive(Debug, Clone)]
pub struct SplitContainer {
    pub tab_id: String,
    pub panels: Vec<SplitPanel>,
    pub active_panel: usize,
}

/// 分屏面板
#[derive(Debug, Clone)]
pub struct SplitPanel {
    pub id: String,
    pub terminal_id: Option<String>,
    pub split_direction: Option<SplitDirection>,
    pub size_percentage: f32,
}

/// 分屏方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl SplitLayout {
    pub fn new() -> Self {
        Self {
            tab_containers: HashMap::new(),
        }
    }

    /// 为标签页创建分屏
    pub fn split(&mut self, tab_id: &str, direction: SplitDirection) -> Result<String, String> {
        let container = self.tab_containers.entry(tab_id.to_string())
            .or_insert_with(|| SplitContainer {
                tab_id: tab_id.to_string(),
                panels: vec![SplitPanel {
                    id: Uuid::new_v4().to_string(),
                    terminal_id: None,
                    split_direction: None,
                    size_percentage: 100.0,
                }],
                active_panel: 0,
            });

        // 创建新面板
        let new_panel = SplitPanel {
            id: Uuid::new_v4().to_string(),
            terminal_id: None,
            split_direction: Some(direction),
            size_percentage: 50.0,
        };

        // 调整现有面板大小
        if let Some(active) = container.panels.get_mut(container.active_panel) {
            active.size_percentage = 50.0;
            active.split_direction = Some(direction);
        }

        let new_id = new_panel.id.clone();
        container.panels.push(new_panel);
        container.active_panel = container.panels.len() - 1;

        Ok(new_id)
    }

    /// 关闭分屏面板
    pub fn close_panel(&mut self, tab_id: &str, panel_id: &str) -> Result<(), String> {
        if let Some(container) = self.tab_containers.get_mut(tab_id) {
            let index = container.panels.iter()
                .position(|p| p.id == panel_id)
                .ok_or_else(|| format!("Panel {} not found", panel_id))?;

            container.panels.remove(index);

            // 重新计算大小
            if !container.panels.is_empty() {
                let new_size = 100.0 / container.panels.len() as f32;
                for panel in &mut container.panels {
                    panel.size_percentage = new_size;
                }
                container.active_panel = 0;
            } else {
                self.tab_containers.remove(tab_id);
            }
        }

        Ok(())
    }
}

impl Default for SplitLayout {
    fn default() -> Self {
        Self::new()
    }
}
