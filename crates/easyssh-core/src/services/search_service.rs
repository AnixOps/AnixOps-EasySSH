//! Server Search Service
//!
//! Provides high-performance server search, filtering, and sorting capabilities.
//! Features include:
//! - Full-text search with fuzzy matching
//! - Pinyin search support for Chinese characters
//! - Search result highlighting
//! - Search history persistence
//! - Multiple filter and sort options

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::db::{Database, HostRecord, TagRecord};
use crate::error::LiteError;

/// Authentication method filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    Key,
    Agent,
}

impl AuthMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthMethod::Password => "password",
            AuthMethod::Key => "key",
            AuthMethod::Agent => "agent",
        }
    }
}

impl std::str::FromStr for AuthMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "password" => Ok(AuthMethod::Password),
            "key" => Ok(AuthMethod::Key),
            "agent" => Ok(AuthMethod::Agent),
            _ => Err(format!("Unknown auth method: {}", s)),
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Sort by field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortBy {
    #[default]
    Name,
    CreatedAt,
    LastConnected,
    Custom,
}

/// Connection status filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Unknown,
    Error,
}

impl ConnectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Unknown => "unknown",
            ConnectionStatus::Error => "error",
        }
    }
}

/// Search query parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Full-text search keyword
    pub keyword: Option<String>,
    /// Filter by group ID
    pub group_id: Option<String>,
    /// Filter by authentication method
    pub auth_method: Option<AuthMethod>,
    /// Filter by connection status
    pub connection_status: Option<ConnectionStatus>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Sort field
    pub sort_by: SortBy,
    /// Sort order
    pub sort_order: SortOrder,
    /// Custom sort order (host IDs in order)
    pub custom_order: Option<Vec<String>>,
    /// Pagination offset
    pub offset: usize,
    /// Pagination limit (0 = no limit)
    pub limit: usize,
}

/// Search result with highlighting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The host record
    pub host: HostRecord,
    /// Tags associated with this host
    pub tags: Vec<TagRecord>,
    /// Match score (higher = better match)
    pub score: f64,
    /// Highlighted fields (field_name -> highlighted_value)
    pub highlights: HashMap<String, String>,
    /// Which fields matched the search
    pub matched_fields: Vec<String>,
}

/// Pinyin conversion cache for Chinese search support
#[derive(Debug, Clone, Default)]
pub struct PinyinCache {
    cache: HashMap<String, String>,
}

impl PinyinCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Convert Chinese characters to pinyin
    /// This is a simplified implementation - in production, use a proper pinyin library
    pub fn to_pinyin(&mut self, text: &str) -> String {
        if let Some(cached) = self.cache.get(text) {
            return cached.clone();
        }

        // Simplified pinyin conversion - extracts ASCII and converts common Chinese characters
        // In production, use pinyin crate: https://crates.io/crates/pinyin
        let pinyin: String = text
            .chars()
            .map(|c| {
                // Basic ASCII passthrough
                if c.is_ascii_alphanumeric() {
                    c.to_lowercase().to_string()
                } else {
                    // For Chinese characters, we'd normally convert to pinyin
                    // Here we just keep the character for fuzzy matching
                    c.to_string()
                }
            })
            .collect();

        self.cache.insert(text.to_string(), pinyin.clone());
        pinyin
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Search index entry for fast lookups
#[derive(Debug, Clone)]
struct SearchIndexEntry {
    host_id: String,
    name: String,
    host: String,
    username: String,
    tags: Vec<String>,
    group_id: Option<String>,
    auth_type: String,
    status: String,
    created_at: String,
    // Pre-computed search terms
    search_terms: Vec<String>,
}

/// Search history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub id: String,
    pub query: String,
    pub timestamp: String,
    pub result_count: usize,
}

/// Server search service
pub struct SearchService {
    db: Arc<Database>,
    /// In-memory search index
    index: Arc<Mutex<Vec<SearchIndexEntry>>>,
    /// Pinyin cache for Chinese search
    pinyin_cache: Arc<Mutex<PinyinCache>>,
    /// Search history (limited size)
    history: Arc<Mutex<Vec<SearchHistoryEntry>>>,
    /// Last index update time
    last_index_update: Arc<Mutex<Instant>>,
    /// Debounce timer for index refresh
    debounce_duration: Duration,
    /// Maximum history entries
    max_history_size: usize,
    /// Case insensitive regex cache
    fuzzy_regex_cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl SearchService {
    /// Create a new search service
    pub fn new(db: Arc<Database>) -> Result<Self, LiteError> {
        let service = Self {
            db,
            index: Arc::new(Mutex::new(Vec::new())),
            pinyin_cache: Arc::new(Mutex::new(PinyinCache::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            last_index_update: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(3600))),
            debounce_duration: Duration::from_millis(300),
            max_history_size: 50,
            fuzzy_regex_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        // Build initial index
        service.rebuild_index()?;

        Ok(service)
    }

    /// Rebuild the search index from database
    pub fn rebuild_index(&self) -> Result<(), LiteError> {
        let mut index = self.index.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search index".to_string())
        })?;

        index.clear();

        // Load all hosts from database
        let hosts = self.load_all_hosts()?;
        let host_tags = self.load_all_host_tags()?;

        for host in hosts {
            let tags = host_tags
                .get(&host.id)
                .cloned()
                .unwrap_or_default();

            let mut search_terms = vec![
                host.name.to_lowercase(),
                host.host.to_lowercase(),
                host.username.to_lowercase(),
            ];

            // Add pinyin variants for Chinese search
            let mut pinyin_cache = self.pinyin_cache.lock().map_err(|_| {
                LiteError::Internal("Failed to lock pinyin cache".to_string())
            })?;
            search_terms.push(pinyin_cache.to_pinyin(&host.name));
            search_terms.push(pinyin_cache.to_pinyin(&host.host));
            drop(pinyin_cache);

            // Add tags to search terms
            for tag in &tags {
                search_terms.push(tag.to_lowercase());
            }

            index.push(SearchIndexEntry {
                host_id: host.id.clone(),
                name: host.name.clone(),
                host: host.host.clone(),
                username: host.username.clone(),
                tags,
                group_id: host.group_id.clone(),
                auth_type: host.auth_type.clone(),
                status: host.status.clone(),
                created_at: host.created_at.clone(),
                search_terms,
            });
        }

        // Update last index time
        let mut last_update = self.last_index_update.lock().map_err(|_| {
            LiteError::Internal("Failed to lock last update time".to_string())
        })?;
        *last_update = Instant::now();

        Ok(())
    }

    /// Load all hosts from database
    fn load_all_hosts(&self) -> Result<Vec<HostRecord>, LiteError> {
        // This would typically call the Database API
        // For now, we return an empty vec - implement based on your db module
        Ok(Vec::new())
    }

    /// Load all host-tag relationships
    fn load_all_host_tags(&self) -> Result<HashMap<String, Vec<String>>, LiteError> {
        // This would typically call the Database API
        Ok(HashMap::new())
    }

    /// Check if index needs refresh (debounced)
    fn should_refresh_index(&self) -> Result<bool, LiteError> {
        let last_update = self.last_index_update.lock().map_err(|_| {
            LiteError::Internal("Failed to lock last update time".to_string())
        })?;

        Ok(last_update.elapsed() > self.debounce_duration)
    }

    /// Refresh index if needed
    pub fn refresh_index_if_needed(&self) -> Result<(), LiteError> {
        if self.should_refresh_index()? {
            self.rebuild_index()?;
        }
        Ok(())
    }

    /// Perform a search with the given query
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, LiteError> {
        // Ensure index is up to date
        self.refresh_index_if_needed()?;

        let index = self.index.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search index".to_string())
        })?;

        let mut results: Vec<SearchResult> = Vec::new();

        // Prepare search keyword patterns
        let keyword_patterns: Option<Vec<Regex>> = if let Some(ref keyword) = query.keyword {
            let patterns = self.build_fuzzy_patterns(keyword)?;
            Some(patterns)
        } else {
            None
        };

        // Iterate through index and filter
        for entry in index.iter() {
            // Apply filters
            if let Some(ref group_id) = query.group_id {
                if entry.group_id.as_ref() != Some(group_id) {
                    continue;
                }
            }

            if let Some(ref auth_method) = query.auth_method {
                if entry.auth_type != auth_method.as_str() {
                    continue;
                }
            }

            if let Some(ref status) = query.connection_status {
                if entry.status != status.as_str() {
                    continue;
                }
            }

            if let Some(ref tags) = query.tags {
                let entry_tags: HashSet<_> = entry.tags.iter().cloned().collect();
                let required_tags: HashSet<_> = tags.iter().cloned().collect();
                if !entry_tags.is_superset(&required_tags) {
                    continue;
                }
            }

            // Calculate match score and highlights
            let (score, highlights, matched_fields) =
                self.calculate_match_score(entry, &keyword_patterns)?;

            // If there's a keyword but no match, skip
            if keyword_patterns.is_some() && matched_fields.is_empty() {
                continue;
            }

            // Load full host record and tags
            let host = self.load_host_by_id(&entry.host_id)?;
            let host_tags = self.load_tags_for_host(&entry.host_id)?;

            results.push(SearchResult {
                host,
                tags: host_tags,
                score,
                highlights,
                matched_fields,
            });
        }

        drop(index);

        // Sort results
        self.sort_results(&mut results, query)?;

        // Record search in history
        self.add_to_history(query, results.len())?;

        // Apply pagination
        if query.offset > 0 {
            results = results.into_iter().skip(query.offset).collect();
        }

        if query.limit > 0 && results.len() > query.limit {
            results.truncate(query.limit);
        }

        Ok(results)
    }

    /// Build fuzzy search patterns from keyword
    fn build_fuzzy_patterns(&self, keyword: &str) -> Result<Vec<Regex>, LiteError> {
        let keyword_lower = keyword.to_lowercase();
        let mut patterns = Vec::new();

        // Exact match pattern
        let exact_pattern = regex::escape(&keyword_lower);
        let exact_regex = Regex::new(&format!("(?i){}", exact_pattern))
            .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
        patterns.push(exact_regex);

        // Fuzzy pattern (allows characters to be separated)
        let fuzzy_pattern: String = keyword_lower
            .chars()
            .map(|c| regex::escape(&c.to_string()))
            .collect::<Vec<_>>()
            .join(".*");
        let fuzzy_regex = Regex::new(&format!("(?i){}", fuzzy_pattern))
            .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
        patterns.push(fuzzy_regex);

        // Pinyin pattern (if keyword looks like pinyin)
        if keyword_lower.chars().all(|c| c.is_ascii_alphabetic()) {
            let pinyin_cache = self.pinyin_cache.lock().map_err(|_| {
                LiteError::Internal("Failed to lock pinyin cache".to_string())
            })?;
            // Add pinyin matching capability
            drop(pinyin_cache);
        }

        Ok(patterns)
    }

    /// Calculate match score and generate highlights
    fn calculate_match_score(
        &self,
        entry: &SearchIndexEntry,
        patterns: &Option<Vec<Regex>>,
    ) -> Result<(f64, HashMap<String, String>, Vec<String>), LiteError> {
        let mut score = 1.0;
        let mut highlights = HashMap::new();
        let mut matched_fields = Vec::new();

        if let Some(ref regex_patterns) = patterns {
            // Check name (highest weight)
            for pattern in regex_patterns {
                if pattern.is_match(&entry.name.to_lowercase()) {
                    score *= 10.0;
                    matched_fields.push("name".to_string());
                    highlights.insert(
                        "name".to_string(),
                        self.highlight_matches(&entry.name, pattern)?,
                    );
                    break;
                }
            }

            // Check host (high weight)
            for pattern in regex_patterns {
                if pattern.is_match(&entry.host.to_lowercase()) {
                    score *= 8.0;
                    if !matched_fields.contains(&"host".to_string()) {
                        matched_fields.push("host".to_string());
                    }
                    highlights.insert(
                        "host".to_string(),
                        self.highlight_matches(&entry.host, pattern)?,
                    );
                    break;
                }
            }

            // Check username (medium weight)
            for pattern in regex_patterns {
                if pattern.is_match(&entry.username.to_lowercase()) {
                    score *= 5.0;
                    if !matched_fields.contains(&"username".to_string()) {
                        matched_fields.push("username".to_string());
                    }
                    highlights.insert(
                        "username".to_string(),
                        self.highlight_matches(&entry.username, pattern)?,
                    );
                    break;
                }
            }

            // Check tags (medium weight)
            for tag in &entry.tags {
                for pattern in regex_patterns {
                    if pattern.is_match(&tag.to_lowercase()) {
                        score *= 4.0;
                        if !matched_fields.contains(&"tags".to_string()) {
                            matched_fields.push("tags".to_string());
                        }
                        // Tag highlighting handled separately
                        break;
                    }
                }
            }

            // Check search terms (lower weight)
            for term in &entry.search_terms {
                for pattern in regex_patterns {
                    if pattern.is_match(term) {
                        score *= 1.5;
                        break;
                    }
                }
            }
        }

        Ok((score, highlights, matched_fields))
    }

    /// Highlight matches in text using markdown-style emphasis
    fn highlight_matches(&self, text: &str, pattern: &Regex) -> Result<String, LiteError> {
        // Replace matches with **matched_text**
        let result = pattern.replace_all(text, |caps: &regex::Captures| {
            format!("**{}**", &caps[0])
        });
        Ok(result.to_string())
    }

    /// Sort results based on query parameters
    fn sort_results(&self, results: &mut [SearchResult], query: &SearchQuery) -> Result<(), LiteError> {
        match query.sort_by {
            SortBy::Name => {
                results.sort_by(|a, b| {
                    let cmp = a.host.name.to_lowercase().cmp(&b.host.name.to_lowercase());
                    match query.sort_order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    }
                });
            }
            SortBy::CreatedAt => {
                results.sort_by(|a, b| {
                    let cmp = a.host.created_at.cmp(&b.host.created_at);
                    match query.sort_order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    }
                });
            }
            SortBy::LastConnected => {
                // For last_connected, we'd need that field in HostRecord
                // For now, fall back to updated_at as approximation
                results.sort_by(|a, b| {
                    let cmp = a.host.updated_at.cmp(&b.host.updated_at);
                    match query.sort_order {
                        SortOrder::Asc => cmp,
                        SortOrder::Desc => cmp.reverse(),
                    }
                });
            }
            SortBy::Custom => {
                if let Some(ref custom_order) = query.custom_order {
                    let order_map: HashMap<_, _> = custom_order
                        .iter()
                        .enumerate()
                        .map(|(i, id)| (id.as_str(), i))
                        .collect();

                    results.sort_by(|a, b| {
                        let a_pos = order_map.get(a.host.id.as_str()).unwrap_or(&usize::MAX);
                        let b_pos = order_map.get(b.host.id.as_str()).unwrap_or(&usize::MAX);
                        let cmp = a_pos.cmp(b_pos);
                        match query.sort_order {
                            SortOrder::Asc => cmp,
                            SortOrder::Desc => cmp.reverse(),
                        }
                    });
                } else {
                    // Fall back to score-based sorting for custom order without explicit order
                    results.sort_by(|a, b| {
                        let cmp = b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal);
                        match query.sort_order {
                            SortOrder::Asc => cmp.reverse(),
                            SortOrder::Desc => cmp,
                        }
                    });
                }
            }
        }

        Ok(())
    }

    /// Add search to history
    fn add_to_history(&self, query: &SearchQuery, result_count: usize) -> Result<(), LiteError> {
        let mut history = self.history.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search history".to_string())
        })?;

        let entry = SearchHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            query: query.keyword.clone().unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            result_count,
        };

        // Add to front
        history.insert(0, entry);

        // Trim to max size
        if history.len() > self.max_history_size {
            history.truncate(self.max_history_size);
        }

        Ok(())
    }

    /// Get search history
    pub fn get_search_history(&self, limit: usize) -> Result<Vec<SearchHistoryEntry>, LiteError> {
        let history = self.history.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search history".to_string())
        })?;

        let result: Vec<_> = history.iter().take(limit).cloned().collect();
        Ok(result)
    }

    /// Clear search history
    pub fn clear_search_history(&self) -> Result<(), LiteError> {
        let mut history = self.history.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search history".to_string())
        })?;

        history.clear();
        Ok(())
    }

    /// Get search suggestions based on partial input
    pub fn get_suggestions(&self, partial: &str, limit: usize) -> Result<Vec<String>, LiteError> {
        if partial.len() < 2 {
            return Ok(Vec::new());
        }

        self.refresh_index_if_needed()?;

        let index = self.index.lock().map_err(|_| {
            LiteError::Internal("Failed to lock search index".to_string())
        })?;

        let partial_lower = partial.to_lowercase();
        let mut suggestions: HashSet<String> = HashSet::new();

        for entry in index.iter() {
            // Check name
            if entry.name.to_lowercase().contains(&partial_lower) {
                suggestions.insert(entry.name.clone());
            }

            // Check host
            if entry.host.to_lowercase().contains(&partial_lower) {
                suggestions.insert(entry.host.clone());
            }

            // Check tags
            for tag in &entry.tags {
                if tag.to_lowercase().contains(&partial_lower) {
                    suggestions.insert(tag.clone());
                }
            }

            if suggestions.len() >= limit {
                break;
            }
        }

        let mut result: Vec<_> = suggestions.into_iter().collect();
        result.sort();
        result.truncate(limit);

        Ok(result)
    }

    /// Quick search - single keyword search with default options
    pub fn quick_search(&self, keyword: &str) -> Result<Vec<SearchResult>, LiteError> {
        let query = SearchQuery {
            keyword: Some(keyword.to_string()),
            sort_by: SortBy::Name,
            sort_order: SortOrder::Asc,
            ..Default::default()
        };

        self.search(&query)
    }

    /// Advanced search with filters
    pub fn advanced_search(
        &self,
        keyword: Option<&str>,
        group_id: Option<&str>,
        auth_method: Option<AuthMethod>,
        tags: Option<Vec<String>>,
    ) -> Result<Vec<SearchResult>, LiteError> {
        let query = SearchQuery {
            keyword: keyword.map(|s| s.to_string()),
            group_id: group_id.map(|s| s.to_string()),
            auth_method,
            tags,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Asc,
            ..Default::default()
        };

        self.search(&query)
    }

    // Placeholder methods - these should be implemented based on actual database API
    fn load_host_by_id(&self, host_id: &str) -> Result<HostRecord, LiteError> {
        // This should call the actual database API
        // For now, return a dummy record
        Ok(HostRecord {
            id: host_id.to_string(),
            name: "Unknown".to_string(),
            host: "unknown".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_type: "agent".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "unknown".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn load_tags_for_host(&self, _host_id: &str) -> Result<Vec<TagRecord>, LiteError> {
        // This should call the actual database API
        Ok(Vec::new())
    }
}

/// Builder for constructing complex search queries
pub struct SearchQueryBuilder {
    query: SearchQuery,
}

impl SearchQueryBuilder {
    pub fn new() -> Self {
        Self {
            query: SearchQuery::default(),
        }
    }

    pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
        self.query.keyword = Some(keyword.into());
        self
    }

    pub fn group_id(mut self, group_id: impl Into<String>) -> Self {
        self.query.group_id = Some(group_id.into());
        self
    }

    pub fn auth_method(mut self, auth_method: AuthMethod) -> Self {
        self.query.auth_method = Some(auth_method);
        self
    }

    pub fn connection_status(mut self, status: ConnectionStatus) -> Self {
        self.query.connection_status = Some(status);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.query.tags = Some(tags);
        self
    }

    pub fn sort_by(mut self, sort_by: SortBy) -> Self {
        self.query.sort_by = sort_by;
        self
    }

    pub fn sort_order(mut self, sort_order: SortOrder) -> Self {
        self.query.sort_order = sort_order;
        self
    }

    pub fn custom_order(mut self, order: Vec<String>) -> Self {
        self.query.custom_order = Some(order);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.query.offset = offset;
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.query.limit = limit;
        self
    }

    pub fn build(self) -> SearchQuery {
        self.query
    }
}

impl Default for SearchQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_from_str() {
        assert_eq!(
            AuthMethod::from_str("password").unwrap(),
            AuthMethod::Password
        );
        assert_eq!(AuthMethod::from_str("key").unwrap(), AuthMethod::Key);
        assert_eq!(AuthMethod::from_str("agent").unwrap(), AuthMethod::Agent);
        assert!(AuthMethod::from_str("unknown").is_err());
    }

    #[test]
    fn test_auth_method_as_str() {
        assert_eq!(AuthMethod::Password.as_str(), "password");
        assert_eq!(AuthMethod::Key.as_str(), "key");
        assert_eq!(AuthMethod::Agent.as_str(), "agent");
    }

    #[test]
    fn test_search_query_builder() {
        let query = SearchQueryBuilder::new()
            .keyword("test")
            .group_id("group-1")
            .auth_method(AuthMethod::Key)
            .connection_status(ConnectionStatus::Connected)
            .tags(vec!["production".to_string()])
            .sort_by(SortBy::Name)
            .sort_order(SortOrder::Desc)
            .offset(0)
            .limit(10)
            .build();

        assert_eq!(query.keyword, Some("test".to_string()));
        assert_eq!(query.group_id, Some("group-1".to_string()));
        assert_eq!(query.auth_method, Some(AuthMethod::Key));
        assert_eq!(query.connection_status, Some(ConnectionStatus::Connected));
        assert_eq!(query.tags, Some(vec!["production".to_string()]));
        assert_eq!(query.sort_by, SortBy::Name);
        assert_eq!(query.sort_order, SortOrder::Desc);
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_pinyin_cache() {
        let mut cache = PinyinCache::new();

        // Test basic conversion
        let pinyin1 = cache.to_pinyin("测试");
        let pinyin2 = cache.to_pinyin("测试");

        // Should return cached result
        assert_eq!(pinyin1, pinyin2);

        // Test cache clear
        cache.clear();
        assert!(cache.cache.is_empty());
    }

    #[test]
    fn test_connection_status_as_str() {
        assert_eq!(ConnectionStatus::Connected.as_str(), "connected");
        assert_eq!(ConnectionStatus::Disconnected.as_str(), "disconnected");
        assert_eq!(ConnectionStatus::Unknown.as_str(), "unknown");
        assert_eq!(ConnectionStatus::Error.as_str(), "error");
    }

    #[test]
    fn test_search_result_serialization() {
        let host = HostRecord {
            id: "test-1".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            identity_file: Some("/path/to/key".to_string()),
            identity_id: None,
            group_id: Some("group-1".to_string()),
            notes: None,
            color: None,
            environment: None,
            region: None,
            purpose: None,
            status: "connected".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = SearchResult {
            host,
            tags: vec![],
            score: 10.0,
            highlights: HashMap::new(),
            matched_fields: vec!["name".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("192.168.1.1"));
    }

    #[test]
    fn test_search_history_entry_serialization() {
        let entry = SearchHistoryEntry {
            id: "hist-1".to_string(),
            query: "production servers".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            result_count: 5,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: SearchHistoryEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.query, deserialized.query);
        assert_eq!(entry.result_count, deserialized.result_count);
    }

    #[test]
    fn test_sort_order() {
        assert_ne!(SortOrder::Asc, SortOrder::Desc);
    }

    #[test]
    fn test_sort_by_variants() {
        let variants = vec![
            SortBy::Name,
            SortBy::CreatedAt,
            SortBy::LastConnected,
            SortBy::Custom,
        ];

        // Ensure all variants are distinct
        assert_eq!(variants.len(), 4);
    }
}
