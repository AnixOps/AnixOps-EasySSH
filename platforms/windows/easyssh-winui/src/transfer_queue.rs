#![allow(dead_code)]

//! Transfer Queue for SFTP File Manager
//! Manages upload/download operations with progress tracking

use std::collections::VecDeque;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransferStatus {
    Queued,
    InProgress { percent: f32 },
    Completed,
    Failed { error: &'static str },
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct TransferItem {
    pub id: String,
    pub file_name: String,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub total_size: u64,
    pub transferred: u64,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub source_path: String,
    pub dest_path: String,
}

impl TransferItem {
    pub fn new(
        id: String,
        file_name: String,
        direction: TransferDirection,
        total_size: u64,
        source_path: String,
        dest_path: String,
    ) -> Self {
        Self {
            id,
            file_name,
            direction,
            status: TransferStatus::Queued,
            total_size,
            transferred: 0,
            start_time: None,
            end_time: None,
            source_path,
            dest_path,
        }
    }

    pub fn progress_percent(&self) -> f32 {
        match self.status {
            TransferStatus::InProgress { percent } => percent,
            TransferStatus::Completed => 100.0,
            _ => 0.0,
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        match self.start_time {
            Some(start) => {
                let end = self.end_time.unwrap_or_else(Instant::now);
                end.duration_since(start).as_secs_f64()
            }
            None => 0.0,
        }
    }

    pub fn transfer_rate(&self) -> f64 {
        let elapsed = self.elapsed_secs();
        if elapsed > 0.0 {
            self.transferred as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn eta_secs(&self) -> Option<f64> {
        if let TransferStatus::InProgress { .. } = self.status {
            let remaining = self.total_size.saturating_sub(self.transferred);
            let rate = self.transfer_rate();
            if rate > 0.0 {
                Some(remaining as f64 / rate)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn direction_icon(&self) -> &'static str {
        match self.direction {
            TransferDirection::Upload => "⬆️",
            TransferDirection::Download => "⬇️",
        }
    }

    pub fn status_icon(&self) -> &'static str {
        match self.status {
            TransferStatus::Queued => "⏳",
            TransferStatus::InProgress { .. } => "🔄",
            TransferStatus::Completed => "✅",
            TransferStatus::Failed { .. } => "❌",
            TransferStatus::Cancelled => "🚫",
        }
    }

    pub fn format_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    pub fn format_rate(bytes_per_sec: f64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        if bytes_per_sec >= GB {
            format!("{:.2} GB/s", bytes_per_sec / GB)
        } else if bytes_per_sec >= MB {
            format!("{:.2} MB/s", bytes_per_sec / MB)
        } else if bytes_per_sec >= KB {
            format!("{:.2} KB/s", bytes_per_sec / KB)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    pub fn status_text(&self) -> String {
        match self.status {
            TransferStatus::Queued => "Queued".to_string(),
            TransferStatus::InProgress { percent } => {
                format!("{:.1}%", percent)
            }
            TransferStatus::Completed => "Completed".to_string(),
            TransferStatus::Failed { error } => format!("Failed: {}", error),
            TransferStatus::Cancelled => "Cancelled".to_string(),
        }
    }
}

pub struct TransferQueue {
    items: VecDeque<TransferItem>,
    next_id: u64,
    max_items: usize,
}

impl TransferQueue {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            next_id: 1,
            max_items: 1000,
        }
    }

    pub fn add(
        &mut self,
        file_name: String,
        total_size: u64,
        direction: TransferDirection,
    ) -> String {
        let id = format!("transfer_{}", self.next_id);
        self.next_id += 1;

        let (source, dest) = match direction {
            TransferDirection::Upload => (
                format!("local/{}", file_name),
                format!("remote/{}", file_name),
            ),
            TransferDirection::Download => (
                format!("remote/{}", file_name),
                format!("local/{}", file_name),
            ),
        };

        let item = TransferItem::new(id.clone(), file_name, direction, total_size, source, dest);

        self.items.push_back(item);

        // Cleanup old completed items if we exceed max
        if self.items.len() > self.max_items {
            self.cleanup_completed();
        }

        id
    }

    pub fn start_transfer(&mut self, id: &str) -> Option<&mut TransferItem> {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = TransferStatus::InProgress { percent: 0.0 };
            item.start_time = Some(Instant::now());
            Some(item)
        } else {
            None
        }
    }

    pub fn update_progress(&mut self, id: &str, transferred: u64) -> Option<f32> {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.transferred = transferred;
            let percent = if item.total_size > 0 {
                (transferred as f32 / item.total_size as f32) * 100.0
            } else {
                0.0
            };
            item.status = TransferStatus::InProgress { percent };
            Some(percent)
        } else {
            None
        }
    }

    pub fn complete_transfer(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = TransferStatus::Completed;
            item.end_time = Some(Instant::now());
            item.transferred = item.total_size;
        }
    }

    pub fn fail_transfer(&mut self, id: &str, error: &'static str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = TransferStatus::Failed { error };
            item.end_time = Some(Instant::now());
        }
    }

    pub fn cancel_transfer(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = TransferStatus::Cancelled;
            item.end_time = Some(Instant::now());
        }
    }

    pub fn remove_item(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }

    pub fn get(&self, id: &str) -> Option<&TransferItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut TransferItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn items(&self) -> &VecDeque<TransferItem> {
        &self.items
    }

    pub fn active_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, TransferStatus::InProgress { .. }))
            .count()
    }

    pub fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, TransferStatus::Queued))
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, TransferStatus::Completed))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, TransferStatus::Failed { .. }))
            .count()
    }

    pub fn total_transferred(&self) -> u64 {
        self.items
            .iter()
            .filter(|i| matches!(i.status, TransferStatus::Completed))
            .map(|i| i.total_size)
            .sum()
    }

    pub fn clear_completed(&mut self) {
        self.items
            .retain(|i| !matches!(i.status, TransferStatus::Completed));
    }

    pub fn clear_all(&mut self) {
        self.items.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    fn cleanup_completed(&mut self) {
        // Keep at most 50 completed items
        let completed: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| matches!(i.status, TransferStatus::Completed))
            .map(|(idx, _)| idx)
            .collect();

        if completed.len() > 50 {
            let to_remove = completed.len() - 50;
            for idx in completed.iter().take(to_remove) {
                if let Some(item) = self.items.get(*idx) {
                    let id = item.id.clone();
                    self.items.retain(|i| i.id != id);
                }
            }
        }
    }

    pub fn get_next_queued(&self) -> Option<&TransferItem> {
        self.items
            .iter()
            .find(|i| matches!(i.status, TransferStatus::Queued))
    }

    pub fn has_active_transfers(&self) -> bool {
        self.items
            .iter()
            .any(|i| matches!(i.status, TransferStatus::InProgress { .. }))
    }

    pub fn overall_progress(&self) -> f32 {
        if self.items.is_empty() {
            return 0.0;
        }

        let total: f32 = self.items.iter().map(|i| i.progress_percent()).sum();
        total / self.items.len() as f32
    }
}

impl Default for TransferQueue {
    fn default() -> Self {
        Self::new()
    }
}
