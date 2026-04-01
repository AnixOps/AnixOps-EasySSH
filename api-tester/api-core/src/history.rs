use crate::types::*;
use std::collections::VecDeque;

pub struct HistoryManager {
    entries: VecDeque<HistoryEntry>,
    max_size: usize,
}

impl HistoryManager {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn add_entry(&mut self, entry: HistoryEntry) {
        // Add to front (most recent first)
        self.entries.push_front(entry);

        // Maintain max size
        while self.entries.len() > self.max_size {
            self.entries.pop_back();
        }
    }

    pub fn get_entries(&self, limit: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().take(limit).collect()
    }

    pub fn get_all_entries(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }

    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.request.name.to_lowercase().contains(&query)
                    || e.request.url.to_lowercase().contains(&query)
                    || e.request.method.to_string().to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn filter_by_status(&self, status_range: (u16, u16)) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let status = e.response.status;
                status >= status_range.0 && status <= status_range.1
            })
            .collect()
    }

    pub fn filter_by_method(&self, method: &HttpMethod) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| &e.request.method == method)
            .collect()
    }

    pub fn filter_by_collection(&self, collection_id: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.collection_id.as_deref() == Some(collection_id))
            .collect()
    }

    pub fn get_recent_by_url(&self, url: &str, limit: usize) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.request.url == url)
            .take(limit)
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn clear_older_than(&mut self, days: i64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        self.entries.retain(|e| e.timestamp > cutoff);
    }

    pub fn delete_entry(&mut self, id: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < initial_len
    }

    pub fn get_stats(&self) -> HistoryStats {
        let total = self.entries.len();
        let mut success_count = 0;
        let mut error_count = 0;
        let mut total_time_ms = 0u64;

        for entry in &self.entries {
            let status = entry.response.status;
            if status >= 200 && status < 300 {
                success_count += 1;
            } else if status >= 400 {
                error_count += 1;
            }
            total_time_ms += entry.response.time_ms;
        }

        let avg_time_ms = if total > 0 {
            total_time_ms / total as u64
        } else {
            0
        };

        // Count unique URLs
        let mut unique_urls = std::collections::HashSet::new();
        for entry in &self.entries {
            unique_urls.insert(&entry.request.url);
        }

        HistoryStats {
            total_requests: total,
            success_count,
            error_count,
            avg_response_time_ms: avg_time_ms,
            unique_endpoints: unique_urls.len(),
        }
    }

    pub fn get_common_urls(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for entry in &self.entries {
            *counts.entry(entry.request.url.clone()).or_insert(0) += 1;
        }

        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted.into_iter().take(limit).collect()
    }

    /// Replay a request from history
    pub fn replay_request(&self, entry_id: &str) -> Option<ApiRequest> {
        self.entries
            .iter()
            .find(|e| e.id == entry_id)
            .map(|e| {
                let mut request = e.request.clone();
                request.id = uuid::Uuid::new_v4().to_string();
                request.name = format!("{} (Replay)", request.name);
                request.created_at = chrono::Utc::now();
                request.updated_at = chrono::Utc::now();
                request
            })
    }

    /// Convert history entry to request for collection
    pub fn history_to_request(&self, entry_id: &str) -> Option<ApiRequest> {
        self.entries
            .iter()
            .find(|e| e.id == entry_id)
            .map(|e| {
                let mut request = e.request.clone();
                request.id = uuid::Uuid::new_v4().to_string();
                request.pre_request_script = None;
                request.test_script = None;
                request
            })
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new(1000) // Default 1000 entries
    }
}

#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_requests: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub avg_response_time_ms: u64,
    pub unique_endpoints: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_entry(id: &str, status: u16) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            request: ApiRequest::new("Test", "https://api.example.com/test"),
            response: ApiResponse {
                status,
                status_text: if status >= 200 && status < 300 { "OK".to_string() } else { "Error".to_string() },
                timestamp: chrono::Utc::now(),
                headers: HashMap::new(),
                body: Vec::new(),
                content_type: None,
                size_bytes: 0,
                time_ms: 100,
            },
            environment_id: None,
            collection_id: None,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_add_and_retrieve() {
        let mut manager = HistoryManager::new(10);
        manager.add_entry(create_test_entry("1", 200));
        manager.add_entry(create_test_entry("2", 404));

        let entries = manager.get_entries(10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "2"); // Most recent first
    }

    #[test]
    fn test_max_size() {
        let mut manager = HistoryManager::new(5);
        for i in 0..10 {
            manager.add_entry(create_test_entry(&i.to_string(), 200));
        }

        assert_eq!(manager.get_entries(100).len(), 5);
    }

    #[test]
    fn test_filter_by_status() {
        let mut manager = HistoryManager::new(10);
        manager.add_entry(create_test_entry("1", 200));
        manager.add_entry(create_test_entry("2", 404));
        manager.add_entry(create_test_entry("3", 500));

        let errors = manager.filter_by_status((400, 599));
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut manager = HistoryManager::new(10);
        manager.add_entry(create_test_entry("1", 200));
        manager.add_entry(create_test_entry("2", 201));
        manager.add_entry(create_test_entry("3", 404));

        let stats = manager.get_stats();
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
    }
}
