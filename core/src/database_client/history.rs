//! Query history management

use crate::database_client::{DatabaseError, DatabaseType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Query history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub connection_id: String,
    pub db_type: DatabaseType,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub rows_returned: usize,
    pub executed_at: DateTime<Utc>,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub title: Option<String>,
    pub is_successful: bool,
    pub error_message: Option<String>,
}

impl QueryHistoryEntry {
    pub fn new(
        connection_id: String,
        db_type: DatabaseType,
        query_text: String,
        execution_time_ms: u64,
        rows_returned: usize,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            connection_id,
            db_type,
            query_text,
            execution_time_ms,
            rows_returned,
            executed_at: Utc::now(),
            is_favorite: false,
            tags: Vec::new(),
            title: None,
            is_successful: true,
            error_message: None,
        }
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.is_successful = false;
        self.error_message = Some(error);
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Generate a title from query if not set
    pub fn generate_title(&mut self) {
        if self.title.is_none() {
            let first_line = self.query_text.lines().next().unwrap_or("").trim();
            let title = if first_line.len() > 50 {
                format!("{}...", &first_line[..50])
            } else {
                first_line.to_string()
            };
            self.title = Some(title);
        }
    }

    /// Get normalized query (for deduplication)
    pub fn normalized_query(&self) -> String {
        self.query_text
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Saved query (favorite)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub query_text: String,
    pub db_type: DatabaseType,
    pub tags: Vec<String>,
    pub parameters: Vec<QueryParameter>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub execution_count: u32,
    pub is_shared: bool,
    pub folder_id: Option<String>,
}

impl SavedQuery {
    pub fn new(title: String, query_text: String, db_type: DatabaseType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            description: None,
            query_text,
            db_type,
            tags: Vec::new(),
            parameters: Vec::new(),
            created_at: now,
            updated_at: now,
            execution_count: 0,
            is_shared: false,
            folder_id: None,
        }
    }

    pub fn from_history(entry: &QueryHistoryEntry) -> Self {
        let mut saved = Self::new(
            entry
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string()),
            entry.query_text.clone(),
            entry.db_type,
        );
        saved.tags = entry.tags.clone();
        saved
    }
}

/// Query parameter for saved queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParameter {
    pub name: String,
    pub data_type: String,
    pub default_value: Option<String>,
    pub is_required: bool,
    pub description: Option<String>,
}

/// Query folder for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// History filter options
#[derive(Debug, Clone, Default)]
pub struct HistoryFilter {
    pub connection_id: Option<String>,
    pub db_type: Option<DatabaseType>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub is_favorite: Option<bool>,
    pub tags: Vec<String>,
    pub search_text: Option<String>,
    pub successful_only: bool,
}

/// Query history manager
pub struct QueryHistoryManager {
    entries: Vec<QueryHistoryEntry>,
    saved_queries: Vec<SavedQuery>,
    folders: Vec<QueryFolder>,
    max_entries: usize,
}

impl QueryHistoryManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            saved_queries: Vec::new(),
            folders: Vec::new(),
            max_entries: 1000,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Add entry to history
    pub async fn add_entry(&mut self, mut entry: QueryHistoryEntry) -> Result<(), DatabaseError> {
        entry.generate_title();

        // Check for duplicates (same normalized query in last hour)
        let normalized = entry.normalized_query();
        let cutoff = Utc::now() - chrono::Duration::hours(1);

        if let Some(existing) = self.entries.iter_mut().find(|e| {
            e.connection_id == entry.connection_id
                && e.normalized_query() == normalized
                && e.executed_at > cutoff
        }) {
            // Update existing entry
            existing.execution_time_ms = entry.execution_time_ms;
            existing.rows_returned = entry.rows_returned;
            existing.executed_at = entry.executed_at;
            existing.is_successful = entry.is_successful;
            existing.error_message = entry.error_message;
        } else {
            self.entries.push(entry);

            // Prune if exceeding max
            if self.entries.len() > self.max_entries {
                self.entries.remove(0);
            }
        }

        Ok(())
    }

    /// Get history entries with filtering
    pub async fn get_entries(
        &self,
        connection_id: Option<&str>,
        limit: usize,
    ) -> Vec<QueryHistoryEntry> {
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| connection_id.map_or(true, |id| e.connection_id == id))
            .cloned()
            .collect();

        // Sort by executed_at descending
        entries.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));

        entries.into_iter().take(limit).collect()
    }

    /// Get filtered history
    pub fn filter_entries(&self, filter: &HistoryFilter) -> Vec<QueryHistoryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                if let Some(ref conn_id) = filter.connection_id {
                    if e.connection_id != *conn_id {
                        return false;
                    }
                }

                if let Some(db_type) = filter.db_type {
                    if e.db_type != db_type {
                        return false;
                    }
                }

                if let Some(ref from) = filter.from_date {
                    if e.executed_at < *from {
                        return false;
                    }
                }

                if let Some(ref to) = filter.to_date {
                    if e.executed_at > *to {
                        return false;
                    }
                }

                if let Some(favorite) = filter.is_favorite {
                    if e.is_favorite != favorite {
                        return false;
                    }
                }

                if !filter.tags.is_empty() {
                    if !filter.tags.iter().all(|t| e.tags.contains(t)) {
                        return false;
                    }
                }

                if filter.successful_only && !e.is_successful {
                    return false;
                }

                if let Some(ref search) = filter.search_text {
                    let search_lower = search.to_lowercase();
                    if !e.query_text.to_lowercase().contains(&search_lower)
                        && !e
                            .title
                            .as_ref()
                            .map_or(false, |t| t.to_lowercase().contains(&search_lower))
                    {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&mut self, entry_id: &str) -> Option<bool> {
        self.entries.iter_mut().find(|e| e.id == entry_id).map(|e| {
            e.is_favorite = !e.is_favorite;
            e.is_favorite
        })
    }

    /// Add tags to entry
    pub fn add_tags(&mut self, entry_id: &str, tags: Vec<String>) -> Option<()> {
        self.entries.iter_mut().find(|e| e.id == entry_id).map(|e| {
            for tag in tags {
                if !e.tags.contains(&tag) {
                    e.tags.push(tag);
                }
            }
        })
    }

    /// Delete entry
    pub fn delete_entry(&mut self, entry_id: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == entry_id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all history
    pub fn clear_history(&mut self) {
        self.entries.clear();
    }

    /// Get statistics
    pub fn get_statistics(&self) -> HistoryStatistics {
        let total_queries = self.entries.len();
        let successful_queries = self.entries.iter().filter(|e| e.is_successful).count();
        let failed_queries = total_queries - successful_queries;

        let avg_execution_time = if total_queries > 0 {
            self.entries
                .iter()
                .map(|e| e.execution_time_ms as u64)
                .sum::<u64>()
                / total_queries as u64
        } else {
            0
        };

        let mut db_type_counts: HashMap<DatabaseType, u32> = HashMap::new();
        for entry in &self.entries {
            *db_type_counts.entry(entry.db_type).or_insert(0) += 1;
        }

        let tag_counts: HashMap<String, u32> = self
            .entries
            .iter()
            .flat_map(|e| e.tags.iter())
            .fold(HashMap::new(), |mut acc, tag| {
                *acc.entry(tag.clone()).or_insert(0) += 1;
                acc
            });

        HistoryStatistics {
            total_queries,
            successful_queries,
            failed_queries,
            favorite_queries: self.entries.iter().filter(|e| e.is_favorite).count(),
            avg_execution_time_ms: avg_execution_time,
            total_rows_returned: self.entries.iter().map(|e| e.rows_returned as u64).sum(),
            queries_today: self
                .entries
                .iter()
                .filter(|e| e.executed_at.date_naive() == Utc::now().date_naive())
                .count(),
            queries_this_week: self
                .entries
                .iter()
                .filter(|e| e.executed_at > Utc::now() - chrono::Duration::weeks(1))
                .count(),
            db_type_distribution: db_type_counts,
            tag_distribution: tag_counts,
        }
    }

    // Saved Queries methods

    /// Save a query
    pub fn save_query(&mut self, query: SavedQuery) {
        // Check if exists
        if let Some(pos) = self.saved_queries.iter().position(|q| q.id == query.id) {
            self.saved_queries[pos] = query;
        } else {
            self.saved_queries.push(query);
        }
    }

    /// Get saved query by ID
    pub fn get_saved_query(&self, id: &str) -> Option<&SavedQuery> {
        self.saved_queries.iter().find(|q| q.id == id)
    }

    /// Delete saved query
    pub fn delete_saved_query(&mut self, id: &str) -> bool {
        if let Some(pos) = self.saved_queries.iter().position(|q| q.id == id) {
            self.saved_queries.remove(pos);
            true
        } else {
            false
        }
    }

    /// List saved queries
    pub fn list_saved_queries(&self, folder_id: Option<&str>) -> Vec<&SavedQuery> {
        self.saved_queries
            .iter()
            .filter(|q| folder_id.map_or(true, |fid| q.folder_id.as_deref() == Some(fid)))
            .collect()
    }

    /// Search saved queries
    pub fn search_saved_queries(&self, search: &str) -> Vec<&SavedQuery> {
        let search_lower = search.to_lowercase();
        self.saved_queries
            .iter()
            .filter(|q| {
                q.title.to_lowercase().contains(&search_lower)
                    || q.query_text.to_lowercase().contains(&search_lower)
                    || q.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&search_lower))
                    || q.description
                        .as_ref()
                        .map_or(false, |d| d.to_lowercase().contains(&search_lower))
            })
            .collect()
    }

    /// Increment execution count
    pub fn increment_execution_count(&mut self, query_id: &str) {
        if let Some(query) = self.saved_queries.iter_mut().find(|q| q.id == query_id) {
            query.execution_count += 1;
            query.updated_at = Utc::now();
        }
    }

    // Folder methods

    /// Create folder
    pub fn create_folder(&mut self, name: String, parent_id: Option<String>) -> QueryFolder {
        let folder = QueryFolder {
            id: Uuid::new_v4().to_string(),
            name,
            parent_id,
            color: None,
            created_at: Utc::now(),
        };
        self.folders.push(folder.clone());
        folder
    }

    /// Delete folder
    pub fn delete_folder(&mut self, id: &str) -> bool {
        if let Some(pos) = self.folders.iter().position(|f| f.id == id) {
            // Move queries to root
            for query in self.saved_queries.iter_mut() {
                if query.folder_id.as_deref() == Some(id) {
                    query.folder_id = None;
                }
            }
            self.folders.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move query to folder
    pub fn move_query_to_folder(
        &mut self,
        query_id: &str,
        folder_id: Option<String>,
    ) -> Option<()> {
        self.saved_queries
            .iter_mut()
            .find(|q| q.id == query_id)
            .map(|q| {
                q.folder_id = folder_id;
            })
    }

    /// Export history to JSON
    pub fn export_to_json(&self) -> Result<String, DatabaseError> {
        #[derive(Serialize)]
        struct ExportData {
            entries: Vec<QueryHistoryEntry>,
            saved_queries: Vec<SavedQuery>,
            folders: Vec<QueryFolder>,
            exported_at: DateTime<Utc>,
        }

        let data = ExportData {
            entries: self.entries.clone(),
            saved_queries: self.saved_queries.clone(),
            folders: self.folders.clone(),
            exported_at: Utc::now(),
        };

        serde_json::to_string_pretty(&data)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))
    }

    /// Import history from JSON
    pub fn import_from_json(&mut self, json: &str) -> Result<(), DatabaseError> {
        #[derive(Deserialize)]
        struct ImportData {
            entries: Option<Vec<QueryHistoryEntry>>,
            saved_queries: Option<Vec<SavedQuery>>,
            folders: Option<Vec<QueryFolder>>,
        }

        let data: ImportData = serde_json::from_str(json)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        if let Some(entries) = data.entries {
            self.entries = entries;
        }

        if let Some(queries) = data.saved_queries {
            self.saved_queries = queries;
        }

        if let Some(folders) = data.folders {
            self.folders = folders;
        }

        Ok(())
    }
}

impl Default for QueryHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// History statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStatistics {
    pub total_queries: usize,
    pub successful_queries: usize,
    pub failed_queries: usize,
    pub favorite_queries: usize,
    pub avg_execution_time_ms: u64,
    pub total_rows_returned: u64,
    pub queries_today: usize,
    pub queries_this_week: usize,
    pub db_type_distribution: HashMap<DatabaseType, u32>,
    pub tag_distribution: HashMap<String, u32>,
}
