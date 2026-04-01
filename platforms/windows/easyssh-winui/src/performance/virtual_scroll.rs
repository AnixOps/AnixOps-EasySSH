#![allow(dead_code)]

/// Virtual Scrolling System for Large Lists
/// Handles 1000+ items with smooth 60fps scrolling

use egui::*;
use std::sync::Arc;
use parking_lot::Mutex;

/// Virtual list state
pub struct VirtualListState {
    /// Total number of items
    item_count: usize,
    /// Height of each item in points
    item_height: f32,
    /// Current scroll offset
    scroll_offset: f32,
    /// Estimated visible item count
    visible_count: usize,
    /// Buffer items above/below visible area
    buffer_items: usize,
}

impl VirtualListState {
    pub fn new(item_count: usize, item_height: f32) -> Self {
        Self {
            item_count,
            item_height,
            scroll_offset: 0.0,
            visible_count: 20,
            buffer_items: 5,
        }
    }

    /// Calculate visible range based on viewport height and scroll position
    pub fn visible_range(&self, viewport_height: f32) -> (usize, usize) {
        let start_idx = (self.scroll_offset / self.item_height).max(0.0) as usize;
        let visible_count = (viewport_height / self.item_height).ceil() as usize;

        let start = start_idx.saturating_sub(self.buffer_items);
        let end = ((start_idx + visible_count + self.buffer_items).min(self.item_count)).max(start);

        (start, end)
    }

    /// Total content height for scrollbar
    pub fn total_height(&self) -> f32 {
        self.item_count as f32 * self.item_height
    }

    /// Update scroll offset
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset.max(0.0);
    }

    /// Update item count (for dynamic lists)
    pub fn set_item_count(&mut self, count: usize) {
        self.item_count = count;
    }
}

/// Virtual list widget for egui
pub struct VirtualList<'a, T> {
    state: &'a mut VirtualListState,
    items: &'a [T],
    item_builder: Box<dyn FnMut(usize, &T, &mut Ui) + 'a>,
    selection: Option<&'a mut usize>,
}

impl<'a, T> VirtualList<'a, T> {
    pub fn new(
        state: &'a mut VirtualListState,
        items: &'a [T],
        item_builder: impl FnMut(usize, &T, &mut Ui) + 'a,
    ) -> Self {
        Self {
            state,
            items,
            item_builder: Box::new(item_builder),
            selection: None,
        }
    }

    pub fn with_selection(mut self, selection: &'a mut usize) -> Self {
        self.selection = Some(selection);
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> Response {
        let available_height = ui.available_height();

        // Update state with current item count
        self.state.set_item_count(self.items.len());

        let (_start, _end) = self.state.visible_range(available_height);

        // Create scroll area with virtual content
        let scroll_offset = self.state.scroll_offset;

        let _output = ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, self.state.item_height, self.items.len(), |ui, row_range| {
                // Only render visible rows
                for idx in row_range {
                    if idx >= self.items.len() {
                        break;
                    }

                    let item = &self.items[idx];
                    let is_selected = self.selection.as_ref().map(|s| **s == idx).unwrap_or(false);

                    // Build the item
                    ui.push_id(idx, |ui| {
                        let (rect, response) = ui.allocate_exact_size(
                            vec2(ui.available_width(), self.state.item_height),
                            Sense::click(),
                        );

                        // Background for selected item
                        if is_selected {
                            ui.painter().rect_filled(
                                rect,
                                Rounding::same(4.0),
                                Color32::from_rgb(0, 122, 255),
                            );
                        }

                        // Render item content
                        let mut child_ui = ui.child_ui(rect, *ui.layout(), None);
                        (self.item_builder)(idx, item, &mut child_ui);

                        // Handle click
                        if response.clicked() {
                            if let Some(ref mut sel) = self.selection {
                                **sel = idx;
                            }
                        }
                    });
                }
            });

        self.state.set_scroll_offset(scroll_offset);
        // Return a dummy response since ScrollAreaOutput doesn't expose response directly in egui 0.28
        ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover())
    }
}

/// Server list specific virtual scrolling implementation
pub struct ServerVirtualList {
    state: VirtualListState,
    filter_text: Arc<Mutex<String>>,
    filtered_indices: Vec<usize>,
}

impl ServerVirtualList {
    pub fn new(total_servers: usize) -> Self {
        Self {
            state: VirtualListState::new(total_servers, 56.0), // 56px per server row
            filter_text: Arc::new(Mutex::new(String::new())),
            filtered_indices: (0..total_servers).collect(),
        }
    }

    /// Update filter and recalculate indices
    pub fn update_filter<F>(&mut self, servers: &[ServerItem], filter: &str, filter_fn: F)
    where
        F: Fn(&ServerItem, &str) -> bool,
    {
        *self.filter_text.lock() = filter.to_string();

        if filter.is_empty() {
            self.filtered_indices = (0..servers.len()).collect();
        } else {
            self.filtered_indices = servers
                .iter()
                .enumerate()
                .filter(|(_, s)| filter_fn(s, filter))
                .map(|(idx, _)| idx)
                .collect();
        }

        self.state.set_item_count(self.filtered_indices.len());
    }

    /// Get filtered item at index
    pub fn get_filtered_item<'a>(&self, servers: &'a [ServerItem], idx: usize) -> Option<&'a ServerItem> {
        self.filtered_indices.get(idx).and_then(|&orig_idx| servers.get(orig_idx))
    }

    /// Render the virtual list
    pub fn render<F>(
        &mut self,
        ui: &mut Ui,
        servers: &[ServerItem],
        mut render_item: F,
    ) -> Vec<Response>
    where
        F: FnMut(&ServerItem, bool, &mut Ui) -> Response,
    {
        let mut responses = Vec::new();
        let available_height = ui.available_height();
        let (start, end) = self.state.visible_range(available_height);

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Allocate space for all items (for scrollbar)
                ui.allocate_space(vec2(ui.available_width(), self.state.total_height()));

                // Create a clip rect for visible items
                let visible_rect = ui.clip_rect();
                let first_visible_y = self.state.scroll_offset;
                let last_visible_y = first_visible_y + available_height;

                // Render only visible items
                for virtual_idx in start..end.min(self.filtered_indices.len()) {
                    let orig_idx = self.filtered_indices[virtual_idx];
                    let item_y = virtual_idx as f32 * self.state.item_height;

                    // Skip if not visible
                    if item_y + self.state.item_height < first_visible_y || item_y > last_visible_y {
                        continue;
                    }

                    if let Some(server) = servers.get(orig_idx) {
                        let item_rect = Rect::from_min_size(
                            pos2(visible_rect.min.x, visible_rect.min.y + item_y - first_visible_y),
                            vec2(visible_rect.width(), self.state.item_height),
                        );

                        let mut child_ui = ui.child_ui(item_rect, *ui.layout(), None);
                        child_ui.set_clip_rect(item_rect);

                        let response = render_item(server, false, &mut child_ui);
                        responses.push(response);
                    }
                }
            });

        responses
    }

    /// Handle scroll events
    pub fn handle_scroll(&mut self, delta: Vec2) {
        self.state.scroll_offset = (self.state.scroll_offset - delta.y).max(0.0);
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.state.set_scroll_offset(offset);
    }
}

/// Server item data for virtual list
#[derive(Clone, Debug)]
pub struct ServerItem {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub is_connected: bool,
    pub is_favorite: bool,
    pub group_name: Option<String>,
}

/// SFTP file list virtual scrolling
pub struct FileVirtualList {
    state: VirtualListState,
}

impl FileVirtualList {
    pub fn new(file_count: usize) -> Self {
        Self {
            state: VirtualListState::new(file_count, 40.0), // 40px per file row
        }
    }

    pub fn update_count(&mut self, count: usize) {
        self.state.set_item_count(count);
    }

    /// Render file list with virtual scrolling
    pub fn render<'a, T, F>(
        &mut self,
        ui: &mut Ui,
        items: &'a [T],
        mut render_item: F,
    ) where
        F: FnMut(&'a T, &mut Ui),
    {
        let available_height = ui.available_height();
        let (_start, _end) = self.state.visible_range(available_height);

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, self.state.item_height, items.len(), |ui, row_range| {
                for idx in row_range {
                    if idx >= items.len() {
                        break;
                    }
                    ui.push_id(idx, |ui| {
                        render_item(&items[idx], ui);
                    });
                }
            });
    }
}

/// Recycle pool for list item widgets (reduces allocations)
pub struct WidgetRecyclePool {
    pool: Arc<Mutex<Vec<WidgetState>>>,
}

struct WidgetState {
    id: Id,
    last_used: Option<std::time::Instant>,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            id: Id::new(0),
            last_used: None,
        }
    }
}

impl WidgetRecyclePool {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Acquire or create widget ID
    pub fn acquire_id(&self, index: usize) -> Id {
        let mut pool = self.pool.lock();

        // Try to reuse
        if let Some(state) = pool.iter_mut().find(|s| s.id.short_debug_format().contains(&format!("{}", index))) {
            state.last_used = Some(std::time::Instant::now());
            state.id
        } else {
            let id = Id::new(index);
            pool.push(WidgetState {
                id,
                last_used: Some(std::time::Instant::now()),
            });
            id
        }
    }

    /// Cleanup old widgets
    pub fn cleanup_stale(&self, max_age: std::time::Duration) {
        let mut pool = self.pool.lock();
        let now = std::time::Instant::now();
        pool.retain(|s| s.last_used.map(|lu| now.duration_since(lu) < max_age).unwrap_or(true));
    }
}

/// Performance-optimized list view with recycling
pub struct RecycledListView<'a, T> {
    items: &'a [T],
    item_height: f32,
    recycle_pool: &'a WidgetRecyclePool,
    scroll_offset: f32,
}

impl<'a, T> RecycledListView<'a, T> {
    pub fn new(
        items: &'a [T],
        item_height: f32,
        recycle_pool: &'a WidgetRecyclePool,
    ) -> Self {
        Self {
            items,
            item_height,
            recycle_pool,
            scroll_offset: 0.0,
        }
    }

    pub fn with_scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn render<F>(&self, ui: &mut Ui, mut render_fn: F)
    where
        F: FnMut(Id, &T, &mut Ui),
    {
        let visible_rect = ui.clip_rect();
        let visible_start_y = self.scroll_offset;
        let visible_end_y = visible_start_y + visible_rect.height();

        let start_idx = (visible_start_y / self.item_height).max(0.0) as usize;
        let end_idx = ((visible_end_y / self.item_height).ceil() as usize + 1)
            .min(self.items.len());

        for idx in start_idx..end_idx {
            let item_y = idx as f32 * self.item_height - self.scroll_offset;
            let item_rect = Rect::from_min_size(
                pos2(visible_rect.min.x, visible_rect.min.y + item_y),
                vec2(visible_rect.width(), self.item_height),
            );

            if let Some(item) = self.items.get(idx) {
                let id = self.recycle_pool.acquire_id(idx);
                let mut child_ui = ui.child_ui(item_rect, *ui.layout(), None);
                child_ui.set_clip_rect(item_rect.intersect(visible_rect));
                render_fn(id, item, &mut child_ui);
            }
        }

        // Request cleanup periodically
        if start_idx % 100 == 0 {
            self.recycle_pool.cleanup_stale(std::time::Duration::from_secs(30));
        }
    }
}
