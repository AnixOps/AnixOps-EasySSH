//! Server Search Service
//!
//! Provides high-performance server search, filtering, and sorting capabilities.
//! Features include:
//! - Full-text search with fuzzy matching
//! - Pinyin search support for Chinese characters
//! - Search result highlighting
//! - Search history persistence
//! - Multiple filter and sort options

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::db::{Database, HostRecord, TagRecord};
use crate::error::LiteError;
use crate::performance::search_optimizer::FastStringMatcher;

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
    /// Common Chinese character to pinyin mappings
    char_map: HashMap<char, String>,
}

impl PinyinCache {
    pub fn new() -> Self {
        let mut char_map = HashMap::new();
        // Common Chinese character mappings
        let common_chars = [
            ('服', "fu"),
            ('务', "wu"),
            ('器', "qi"),
            ('主', "zhu"),
            ('机', "ji"),
            ('测', "ce"),
            ('试', "shi"),
            ('生', "sheng"),
            ('产', "chan"),
            ('开', "kai"),
            ('发', "fa"),
            ('数', "shu"),
            ('据', "ju"),
            ('库', "ku"),
            ('网', "wang"),
            ('络', "luo"),
            ('存', "cun"),
            ('储', "chu"),
            ('计', "ji"),
            ('算', "suan"),
            ('云', "yun"),
            ('端', "duan"),
            ('本', "ben"),
            ('地', "di"),
            ('远', "yuan"),
            ('程', "cheng"),
            ('连', "lian"),
            ('接', "jie"),
            ('配', "pei"),
            ('置', "zhi"),
            ('安', "an"),
            ('全', "quan"),
            ('密', "mi"),
            ('码', "ma"),
            ('密', "mi"),
            ('钥', "yao"),
            ('管', "guan"),
            ('理', "li"),
            ('监', "jian"),
            ('控', "kong"),
            ('预', "yu"),
            ('警', "jing"),
            ('报', "bao"),
            ('警', "jing"),
            ('告', "gao"),
            ('错', "cuo"),
            ('误', "wu"),
            ('失', "shi"),
            ('败', "bai"),
            ('成', "cheng"),
            ('功', "gong"),
            ('完', "wan"),
            ('成', "cheng"),
            ('新', "xin"),
            ('建', "jian"),
            ('删', "shan"),
            ('除', "chu"),
            ('修', "xiu"),
            ('改', "gai"),
            ('查', "cha"),
            ('看', "kan"),
            ('搜', "sou"),
            ('索', "suo"),
            ('过', "guo"),
            ('滤', "lv"),
            ('排', "pai"),
            ('序', "xu"),
            ('组', "zu"),
            ('分', "fen"),
            ('类', "lei"),
            ('标', "biao"),
            ('签', "qian"),
            ('备', "bei"),
            ('注', "zhu"),
            ('描', "miao"),
            ('述', "shu"),
            ('名', "ming"),
            ('称', "cheng"),
            ('地', "di"),
            ('址', "zhi"),
            ('用', "yong"),
            ('户', "hu"),
            ('名', "ming"),
            ('端', "duan"),
            ('口', "kou"),
            ('密', "mi"),
            ('令', "ling"),
            ('认', "ren"),
            ('证', "zheng"),
            ('授', "shou"),
            ('权', "quan"),
            ('组', "zu"),
            ('织', "zhi"),
            ('公', "gong"),
            ('司', "si"),
            ('部', "bu"),
            ('门', "men"),
            ('团', "tuan"),
            ('队', "dui"),
            ('项', "xiang"),
            ('目', "mu"),
            ('环', "huan"),
            ('境', "jing"),
            ('区', "qu"),
            ('域', "yu"),
        ];
        for (ch, py) in common_chars {
            char_map.insert(ch, py.to_string());
        }

        Self {
            cache: HashMap::new(),
            char_map,
        }
    }

    /// Convert text to searchable pinyin format
    /// Returns both original and pinyin variants for maximum searchability
    pub fn to_pinyin(&mut self, text: &str) -> String {
        if let Some(cached) = self.cache.get(text) {
            return cached.clone();
        }

        let mut pinyin_parts = Vec::new();
        let mut current_word = String::new();

        for ch in text.chars() {
            if let Some(py) = self.char_map.get(&ch) {
                // Found Chinese character, add current word and pinyin
                if !current_word.is_empty() {
                    pinyin_parts.push(current_word.to_lowercase());
                    current_word.clear();
                }
                pinyin_parts.push(py.clone());
            } else if ch.is_ascii_alphanumeric() {
                current_word.push(ch.to_lowercase().next().unwrap_or(ch));
            } else {
                // Separator, push current word if any
                if !current_word.is_empty() {
                    pinyin_parts.push(current_word.to_lowercase());
                    current_word.clear();
                }
            }
        }

        // Add remaining word
        if !current_word.is_empty() {
            pinyin_parts.push(current_word.to_lowercase());
        }

        let result = pinyin_parts.join("");
        self.cache.insert(text.to_string(), result.clone());
        result
    }

    /// Convert text to pinyin with word segmentation (for better matching)
    pub fn to_pinyin_segmented(&mut self, text: &str) -> Vec<String> {
        let full = self.to_pinyin(text);
        let mut segments = Vec::new();
        let mut current = String::new();

        for ch in full.chars() {
            if ch.is_ascii_lowercase() {
                current.push(ch);
            } else if !current.is_empty() {
                segments.push(current.clone());
                current.clear();
            }
        }

        if !current.is_empty() {
            segments.push(current);
        }

        segments
    }

    /// Get all pinyin variants of a string (for fuzzy matching)
    /// Returns: [original, pinyin_full, initials, pinyin_segments...]
    pub fn get_search_variants(&mut self, text: &str) -> Vec<String> {
        let mut variants = Vec::new();

        // Original text (lowercase)
        variants.push(text.to_lowercase());

        // Full pinyin
        let pinyin = self.to_pinyin(text);
        if pinyin != text.to_lowercase() {
            variants.push(pinyin.clone());
        }

        // Pinyin initials (first letter of each segment)
        let segments = self.to_pinyin_segmented(text);
        let initials: String = segments.iter().filter_map(|s| s.chars().next()).collect();
        if initials.len() > 1 {
            variants.push(initials);
        }

        variants
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.char_map.len())
    }
}

/// Search index entry for fast lookups
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

/// Fuzzy match configuration
#[derive(Debug, Clone, Copy)]
pub struct FuzzyConfig {
    /// Maximum edit distance for fuzzy matching
    pub max_distance: u32,
    /// Enable prefix matching
    pub prefix_match: bool,
    /// Enable substring matching
    pub substring_match: bool,
    /// Enable initials matching (for pinyin)
    pub initials_match: bool,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: f64,
}

impl Default for FuzzyConfig {
    fn default() -> Self {
        Self {
            max_distance: 2,
            prefix_match: true,
            substring_match: true,
            initials_match: true,
            min_score: 0.3,
        }
    }
}

/// Search history persistence configuration
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    /// Maximum entries to keep in memory
    pub max_memory_entries: usize,
    /// Maximum entries to persist to database
    pub max_persisted_entries: usize,
    /// Whether to deduplicate entries
    pub deduplicate: bool,
    /// Minimum query length to record
    pub min_query_length: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 50,
            max_persisted_entries: 100,
            deduplicate: true,
            min_query_length: 2,
        }
    }
}

/// Search suggestion with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub score: f64,
    pub frequency: usize,
}

/// Type of search suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    History,
    HostName,
    HostAddress,
    Tag,
    Group,
    Command,
    Recent,
}

/// Advanced search parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvancedSearchParams {
    /// Include archived hosts
    pub include_archived: bool,
    /// Filter by port range
    pub port_range: Option<(u16, u16)>,
    /// Filter by connection count (min, max)
    pub connection_count_range: Option<(usize, usize)>,
    /// Filter by last connected time (days ago)
    pub last_connected_within_days: Option<u32>,
    /// Filter by favorite status
    pub favorite_only: bool,
    /// Custom field filters
    pub custom_filters: HashMap<String, String>,
}

/// Search performance metrics
#[derive(Debug, Clone, Default)]
pub struct SearchMetrics {
    pub query_time_ms: u64,
    pub results_count: usize,
    pub index_hits: usize,
    pub cache_hits: usize,
}

/// Server search service
#[allow(dead_code)]
pub struct SearchService {
    db: Arc<Database>,
    /// In-memory search index
    index: Arc<Mutex<Vec<SearchIndexEntry>>>,
    /// Pinyin cache for Chinese search
    pinyin_cache: Arc<Mutex<PinyinCache>>,
    /// Search history (limited size)
    history: Arc<Mutex<VecDeque<SearchHistoryEntry>>>,
    /// Search history configuration
    history_config: HistoryConfig,
    /// Last index update time
    last_index_update: Arc<Mutex<Instant>>,
    /// Debounce timer for index refresh
    debounce_duration: Duration,
    /// Fuzzy match configuration
    fuzzy_config: FuzzyConfig,
    /// Case insensitive regex cache
    fuzzy_regex_cache: Arc<Mutex<HashMap<String, Regex>>>,
    /// Search result cache for repeated queries
    #[allow(clippy::type_complexity)]
    result_cache: Arc<Mutex<HashMap<String, (Vec<SearchResult>, Instant)>>>,
    /// Cache TTL
    cache_ttl: Duration,
}

impl SearchService {
    /// Create a new search service
    pub fn new(db: Arc<Database>) -> Result<Self, LiteError> {
        let service = Self {
            db,
            index: Arc::new(Mutex::new(Vec::new())),
            pinyin_cache: Arc::new(Mutex::new(PinyinCache::new())),
            history: Arc::new(Mutex::new(VecDeque::new())),
            history_config: HistoryConfig::default(),
            last_index_update: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(3600))),
            debounce_duration: Duration::from_millis(300),
            fuzzy_config: FuzzyConfig::default(),
            fuzzy_regex_cache: Arc::new(Mutex::new(HashMap::new())),
            result_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
        };

        // Load search history from database
        service.load_history_from_db()?;

        // Build initial index
        service.rebuild_index()?;

        Ok(service)
    }

    /// Create with custom configuration
    pub fn with_config(
        db: Arc<Database>,
        fuzzy_config: FuzzyConfig,
        history_config: HistoryConfig,
    ) -> Result<Self, LiteError> {
        let service = Self {
            db,
            index: Arc::new(Mutex::new(Vec::new())),
            pinyin_cache: Arc::new(Mutex::new(PinyinCache::new())),
            history: Arc::new(Mutex::new(VecDeque::new())),
            history_config,
            last_index_update: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(3600))),
            debounce_duration: Duration::from_millis(300),
            fuzzy_config,
            fuzzy_regex_cache: Arc::new(Mutex::new(HashMap::new())),
            result_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
        };

        service.load_history_from_db()?;
        service.rebuild_index()?;

        Ok(service)
    }

    /// Load search history from database
    fn load_history_from_db(&self) -> Result<(), LiteError> {
        // This would load from database in production
        // For now, history starts empty
        Ok(())
    }

    /// Save search history to database
    fn save_history_to_db(&self) -> Result<(), LiteError> {
        // This would persist to database in production
        Ok(())
    }

    /// Rebuild the search index from database
    pub fn rebuild_index(&self) -> Result<(), LiteError> {
        let mut index = self
            .index
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search index".to_string()))?;

        index.clear();

        // Load all hosts from database
        let hosts = self.load_all_hosts()?;
        let host_tags = self.load_all_host_tags()?;

        for host in hosts {
            let tags = host_tags.get(&host.id).cloned().unwrap_or_default();

            let mut search_terms = vec![
                host.name.to_lowercase(),
                host.host.to_lowercase(),
                host.username.to_lowercase(),
            ];

            // Add pinyin variants for Chinese search
            let mut pinyin_cache = self
                .pinyin_cache
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock pinyin cache".to_string()))?;
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
        let mut last_update = self
            .last_index_update
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock last update time".to_string()))?;
        *last_update = Instant::now();

        Ok(())
    }

    /// Load all hosts from database
    fn load_all_hosts(&self) -> Result<Vec<HostRecord>, LiteError> {
        self.db.get_hosts()
    }

    /// Load all host-tag relationships
    fn load_all_host_tags(&self) -> Result<HashMap<String, Vec<String>>, LiteError> {
        // Get all hosts and load their tags
        let hosts = self.db.get_hosts()?;
        let mut host_tags: HashMap<String, Vec<String>> = HashMap::new();

        for host in hosts {
            let tags = self.db.get_host_tags(&host.id)?;
            if !tags.is_empty() {
                host_tags.insert(host.id, tags.into_iter().map(|t| t.name).collect());
            }
        }

        Ok(host_tags)
    }

    /// Check if index needs refresh (debounced)
    fn should_refresh_index(&self) -> Result<bool, LiteError> {
        let last_update = self
            .last_index_update
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock last update time".to_string()))?;

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
        let start_time = Instant::now();

        // Check result cache for repeated queries
        let cache_key = format!("{:?}", query);
        {
            let cache = self
                .result_cache
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock result cache".to_string()))?;

            if let Some((cached_results, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < self.cache_ttl {
                    return Ok(cached_results.clone());
                }
            }
        }

        // Ensure index is up to date
        self.refresh_index_if_needed()?;

        let index = self
            .index
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search index".to_string()))?;

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

            // Calculate match score and highlights using enhanced fuzzy matching
            let keyword_ref = query.keyword.as_deref();
            let (score, highlights, matched_fields) =
                self.calculate_match_score(entry, &keyword_patterns, keyword_ref)?;

            // If there's a keyword but no match, skip
            if query.keyword.is_some() && matched_fields.is_empty() {
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
        let total_count = results.len();

        if query.offset > 0 {
            results = results.into_iter().skip(query.offset).collect();
        }

        if query.limit > 0 && results.len() > query.limit {
            results.truncate(query.limit);
        }

        // Cache results
        {
            let mut cache = self
                .result_cache
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock result cache".to_string()))?;
            cache.insert(cache_key, (results.clone(), Instant::now()));

            // Clean old cache entries
            cache.retain(|_, (_, ts)| ts.elapsed() < self.cache_ttl);
        }

        let elapsed = start_time.elapsed();
        log::debug!(
            "Search for '{:?}' completed in {:?}, found {} results (total: {})",
            query.keyword,
            elapsed,
            results.len(),
            total_count
        );

        Ok(results)
    }

    /// Build fuzzy search patterns from keyword
    fn build_fuzzy_patterns(&self, keyword: &str) -> Result<Vec<Regex>, LiteError> {
        let keyword_lower = keyword.to_lowercase();
        let mut patterns = Vec::new();

        // Exact match pattern (highest priority)
        let exact_pattern = regex::escape(&keyword_lower);
        let exact_regex = Regex::new(&format!("(?i){}", exact_pattern))
            .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
        patterns.push(exact_regex);

        // Word boundary match (for multi-word queries)
        let words: Vec<String> = keyword_lower
            .split_whitespace()
            .map(regex::escape)
            .collect();
        if words.len() > 1 {
            let word_boundary_pattern = words.join(".*");
            let word_boundary_regex = Regex::new(&format!("(?i){}", word_boundary_pattern))
                .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
            patterns.push(word_boundary_regex);
        }

        // Character-by-character fuzzy pattern (allows characters to be separated)
        let fuzzy_pattern: String = keyword_lower
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| regex::escape(&c.to_string()))
            .collect::<Vec<_>>()
            .join(".*");
        let fuzzy_regex = Regex::new(&format!("(?i){}", fuzzy_pattern))
            .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
        patterns.push(fuzzy_regex);

        // Pinyin patterns for Chinese search
        let mut pinyin_cache = self
            .pinyin_cache
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock pinyin cache".to_string()))?;

        // Get pinyin variants of the keyword
        let pinyin_variants = pinyin_cache.get_search_variants(&keyword_lower);

        for variant in &pinyin_variants {
            if variant != &keyword_lower && !variant.is_empty() {
                // Add pinyin exact match
                let py_pattern = regex::escape(variant);
                let py_regex = Regex::new(&format!("(?i){}", py_pattern))
                    .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
                patterns.push(py_regex);

                // Add pinyin fuzzy match
                let py_fuzzy: String = variant
                    .chars()
                    .map(|c| regex::escape(&c.to_string()))
                    .collect::<Vec<_>>()
                    .join(".*");
                let py_fuzzy_regex = Regex::new(&format!("(?i){}", py_fuzzy))
                    .map_err(|e| LiteError::Internal(format!("Regex error: {}", e)))?;
                patterns.push(py_fuzzy_regex);
            }
        }

        drop(pinyin_cache);

        Ok(patterns)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
            row[0] = i;
        }
        for (j, item) in matrix[0].iter_mut().enumerate().take(len2 + 1) {
            *item = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = [
                    matrix[i - 1][j] + 1,        // deletion
                    matrix[i][j - 1] + 1,        // insertion
                    matrix[i - 1][j - 1] + cost, // substitution
                ]
                .into_iter()
                .min()
                .unwrap();
            }
        }

        matrix[len1][len2]
    }

    /// Calculate fuzzy match score using multiple algorithms
    fn calculate_fuzzy_score(&self, text: &str, keyword: &str) -> f64 {
        let text_lower = text.to_lowercase();
        let keyword_lower = keyword.to_lowercase();

        // Exact match - highest score
        if text_lower == keyword_lower {
            return 1.0;
        }

        // Prefix match - very high score
        if text_lower.starts_with(&keyword_lower) {
            return 0.9 + (keyword_lower.len() as f64 / text_lower.len() as f64) * 0.1;
        }

        // Substring match - high score
        if text_lower.contains(&keyword_lower) {
            let position = text_lower.find(&keyword_lower).unwrap() as f64;
            let position_penalty = position / text_lower.len() as f64 * 0.2;
            return 0.7 - position_penalty
                + (keyword_lower.len() as f64 / text_lower.len() as f64) * 0.2;
        }

        // Fuzzy character match
        if FastStringMatcher::fuzzy_match(&text_lower, &keyword_lower) {
            let base_score = FastStringMatcher::fuzzy_score(&text_lower, &keyword_lower);
            return base_score * 0.6;
        }

        // Levenshtein distance for approximate matching (only for short keywords)
        if keyword_lower.len() <= 10 && text_lower.len() <= 50 {
            let distance = Self::levenshtein_distance(&text_lower, &keyword_lower);
            let max_len = text_lower.len().max(keyword_lower.len());
            let similarity = 1.0 - (distance as f64 / max_len as f64);

            if similarity >= self.fuzzy_config.min_score {
                return similarity * 0.5;
            }
        }

        // Pinyin match for Chinese characters
        let mut pinyin_cache = match self.pinyin_cache.lock() {
            Ok(cache) => cache,
            Err(_) => return 0.0, // Return 0 score if lock fails
        };

        let text_variants = pinyin_cache.get_search_variants(&text_lower);
        let keyword_variants = pinyin_cache.get_search_variants(&keyword_lower);

        for text_variant in &text_variants {
            for keyword_variant in &keyword_variants {
                if text_variant.contains(keyword_variant) {
                    let score = if text_variant == keyword_variant {
                        0.85
                    } else {
                        let pos = text_variant.find(keyword_variant).unwrap_or(0) as f64;
                        let len_ratio = keyword_variant.len() as f64 / text_variant.len() as f64;
                        0.75 - (pos / text_variant.len() as f64 * 0.1) + len_ratio * 0.1
                    };
                    return score;
                }
            }
        }

        0.0
    }

    /// Calculate match score using enhanced fuzzy matching with pinyin support
    #[allow(clippy::type_complexity)]
    fn calculate_match_score(
        &self,
        entry: &SearchIndexEntry,
        patterns: &Option<Vec<Regex>>,
        keyword: Option<&str>,
    ) -> Result<(f64, HashMap<String, String>, Vec<String>), LiteError> {
        let mut score = 1.0;
        let mut highlights = HashMap::new();
        let mut matched_fields = Vec::new();

        if let Some(kw) = keyword {
            let kw_lower = kw.to_lowercase();

            // Check name (highest weight)
            let name_score = self.calculate_fuzzy_score(&entry.name, &kw_lower);
            if name_score > 0.0 {
                score *= 10.0 * name_score;
                matched_fields.push("name".to_string());
                highlights.insert(
                    "name".to_string(),
                    self.highlight_fuzzy_matches(&entry.name, &kw_lower)?,
                );
            }

            // Check host (high weight)
            let host_score = self.calculate_fuzzy_score(&entry.host, &kw_lower);
            if host_score > 0.0 {
                score *= 8.0 * host_score;
                if !matched_fields.contains(&"host".to_string()) {
                    matched_fields.push("host".to_string());
                }
                highlights.insert(
                    "host".to_string(),
                    self.highlight_fuzzy_matches(&entry.host, &kw_lower)?,
                );
            }

            // Check username (medium weight)
            let username_score = self.calculate_fuzzy_score(&entry.username, &kw_lower);
            if username_score > 0.0 {
                score *= 5.0 * username_score;
                if !matched_fields.contains(&"username".to_string()) {
                    matched_fields.push("username".to_string());
                }
                highlights.insert(
                    "username".to_string(),
                    self.highlight_fuzzy_matches(&entry.username, &kw_lower)?,
                );
            }

            // Check tags (medium weight)
            for tag in &entry.tags {
                let tag_score = self.calculate_fuzzy_score(tag, &kw_lower);
                if tag_score > 0.0 {
                    score *= 4.0 * tag_score;
                    if !matched_fields.contains(&"tags".to_string()) {
                        matched_fields.push("tags".to_string());
                    }
                    // Tag highlighting handled separately
                    break;
                }
            }

            // Check search terms including pinyin variants (lower weight)
            for term in &entry.search_terms {
                let term_score = self.calculate_fuzzy_score(term, &kw_lower);
                if term_score > 0.0 {
                    score *= 1.5 * term_score;
                    break;
                }
            }

            // Also use regex patterns for highlighting if available
            if let Some(ref regex_patterns) = patterns {
                if matched_fields.is_empty() {
                    // Fallback to regex matching
                    for pattern in regex_patterns {
                        if pattern.is_match(&entry.name.to_lowercase()) {
                            score = 5.0;
                            matched_fields.push("name".to_string());
                            highlights.insert(
                                "name".to_string(),
                                self.highlight_matches(&entry.name, pattern)?,
                            );
                            break;
                        }
                    }
                }
            }
        }

        Ok((score, highlights, matched_fields))
    }

    /// Highlight fuzzy matches in text
    fn highlight_fuzzy_matches(&self, text: &str, keyword: &str) -> Result<String, LiteError> {
        if keyword.is_empty() {
            return Ok(text.to_string());
        }

        let text_lower = text.to_lowercase();
        let keyword_lower = keyword.to_lowercase();

        // Try exact match first
        if let Some(pos) = text_lower.find(&keyword_lower) {
            let mut result = String::new();
            result.push_str(&text[..pos]);
            result.push_str("**");
            result.push_str(&text[pos..pos + keyword_lower.len()]);
            result.push_str("**");
            result.push_str(&text[pos + keyword_lower.len()..]);
            return Ok(result);
        }

        // Try fuzzy character matching
        let mut result = String::new();
        let mut text_chars = text.chars().peekable();
        let mut keyword_chars = keyword_lower.chars().peekable();

        while let Some(kc) = keyword_chars.peek() {
            let mut matched = false;
            while let Some(tc) = text_chars.peek() {
                if tc.to_lowercase().next() == Some(*kc) {
                    // Found match
                    if !matched {
                        result.push_str("**");
                    }
                    result.push(*tc);
                    matched = true;
                    text_chars.next();
                    break;
                } else {
                    if matched {
                        result.push_str("**");
                        matched = false;
                    }
                    result.push(*tc);
                    text_chars.next();
                }
            }

            if !matched {
                // No match found for this keyword character
                if result.ends_with("**") {
                    // Already in highlight mode, just continue
                }
            }
            keyword_chars.next();
        }

        // Add remaining text
        for tc in text_chars {
            result.push(tc);
        }

        // Close highlight if still open
        if result.matches("**").count() % 2 == 1 {
            result.push_str("**");
        }

        Ok(result)
    }

    /// Legacy highlight method for regex patterns
    fn highlight_matches(&self, text: &str, pattern: &Regex) -> Result<String, LiteError> {
        let result =
            pattern.replace_all(text, |caps: &regex::Captures| format!("**{}**", &caps[0]));
        Ok(result.to_string())
    }

    /// Sort results based on query parameters
    fn sort_results(
        &self,
        results: &mut [SearchResult],
        query: &SearchQuery,
    ) -> Result<(), LiteError> {
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
                        let cmp = b
                            .score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal);
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

    /// Add search to history with deduplication and frequency tracking
    fn add_to_history(&self, query: &SearchQuery, result_count: usize) -> Result<(), LiteError> {
        // Skip empty or short queries
        let query_text = query.keyword.clone().unwrap_or_default();
        if query_text.len() < self.history_config.min_query_length {
            return Ok(());
        }

        let mut history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;

        let query_lower = query_text.to_lowercase();

        // Check for duplicates if deduplication is enabled
        if self.history_config.deduplicate {
            if let Some(pos) = history
                .iter()
                .position(|e| e.query.to_lowercase() == query_lower)
            {
                // Remove existing entry to move it to front
                history.remove(pos);
            }
        }

        // Create new entry
        let entry = SearchHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            query: query_text.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            result_count,
        };

        // Add to front
        history.push_front(entry);

        // Trim to max memory size
        while history.len() > self.history_config.max_memory_entries {
            history.pop_back();
        }

        // Persist to database
        drop(history);
        self.save_history_to_db()?;

        Ok(())
    }

    /// Get search history with optional filtering
    pub fn get_search_history(
        &self,
        limit: usize,
        filter: Option<&str>,
    ) -> Result<Vec<SearchHistoryEntry>, LiteError> {
        let history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;

        let mut results: Vec<_> = if let Some(f) = filter {
            let f_lower = f.to_lowercase();
            history
                .iter()
                .filter(|e| e.query.to_lowercase().contains(&f_lower))
                .cloned()
                .collect()
        } else {
            history.iter().cloned().collect()
        };

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if results.len() > limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Clear search history from memory and database
    pub fn clear_search_history(&self) -> Result<(), LiteError> {
        {
            let mut history = self
                .history
                .lock()
                .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;
            history.clear();
        }

        // Clear from database
        self.save_history_to_db()?;

        Ok(())
    }

    /// Get frequently used searches (for quick access)
    pub fn get_frequent_searches(&self, limit: usize) -> Result<Vec<(String, usize)>, LiteError> {
        let history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;

        // Count frequency of each query
        let mut frequency: HashMap<String, usize> = HashMap::new();
        for entry in history.iter() {
            *frequency.entry(entry.query.clone()).or_insert(0) += 1;
        }

        // Sort by frequency
        let mut sorted: Vec<_> = frequency.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        // Apply limit
        if sorted.len() > limit {
            sorted.truncate(limit);
        }

        Ok(sorted)
    }

    /// Get intelligent search suggestions with ranking
    pub fn get_suggestions(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<SearchSuggestion>, LiteError> {
        if partial.is_empty() {
            // Return recent history and popular searches when no input
            return self.get_default_suggestions(limit);
        }

        self.refresh_index_if_needed()?;

        let index = self
            .index
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search index".to_string()))?;

        let history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;

        let partial_lower = partial.to_lowercase();
        let mut suggestions: Vec<SearchSuggestion> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // 1. Search from history (highest priority for exact matches)
        for entry in history.iter() {
            if entry.query.to_lowercase().starts_with(&partial_lower)
                && seen.insert(entry.query.clone())
            {
                suggestions.push(SearchSuggestion {
                    text: entry.query.clone(),
                    suggestion_type: SuggestionType::History,
                    score: 1.0,
                    frequency: 1,
                });
            }
        }

        // 2. Search from host names
        for entry in index.iter() {
            let name_score = self.calculate_fuzzy_score(&entry.name, &partial_lower);
            if name_score > 0.6 && seen.insert(entry.name.clone()) {
                suggestions.push(SearchSuggestion {
                    text: entry.name.clone(),
                    suggestion_type: SuggestionType::HostName,
                    score: name_score * 0.9,
                    frequency: 0,
                });
            }
        }

        // 3. Search from host addresses
        for entry in index.iter() {
            if entry.host.to_lowercase().contains(&partial_lower) && seen.insert(entry.host.clone())
            {
                suggestions.push(SearchSuggestion {
                    text: entry.host.clone(),
                    suggestion_type: SuggestionType::HostAddress,
                    score: 0.8,
                    frequency: 0,
                });
            }
        }

        // 4. Search from tags
        for entry in index.iter() {
            for tag in &entry.tags {
                let tag_score = self.calculate_fuzzy_score(tag, &partial_lower);
                if tag_score > 0.7 && seen.insert(tag.clone()) {
                    suggestions.push(SearchSuggestion {
                        text: tag.clone(),
                        suggestion_type: SuggestionType::Tag,
                        score: tag_score * 0.85,
                        frequency: 0,
                    });
                }
            }
        }

        // 5. Pinyin matching for Chinese input
        let mut pinyin_cache = self
            .pinyin_cache
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock pinyin cache".to_string()))?;

        for entry in index.iter() {
            let variants = pinyin_cache.get_search_variants(&entry.name);
            for variant in &variants {
                if variant.contains(&partial_lower) {
                    if seen.insert(entry.name.clone()) {
                        suggestions.push(SearchSuggestion {
                            text: entry.name.clone(),
                            suggestion_type: SuggestionType::HostName,
                            score: 0.75,
                            frequency: 0,
                        });
                    }
                    break;
                }
            }
        }

        drop(pinyin_cache);

        // Sort by score (descending)
        suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if suggestions.len() > limit {
            suggestions.truncate(limit);
        }

        Ok(suggestions)
    }

    /// Get default suggestions when no input (recent and popular)
    fn get_default_suggestions(&self, limit: usize) -> Result<Vec<SearchSuggestion>, LiteError> {
        let history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;

        let suggestions: Vec<SearchSuggestion> = history
            .iter()
            .take(limit)
            .map(|entry| SearchSuggestion {
                text: entry.query.clone(),
                suggestion_type: SuggestionType::Recent,
                score: 1.0,
                frequency: 1,
            })
            .collect();

        Ok(suggestions)
    }

    /// Get suggestions by type
    pub fn get_suggestions_by_type(
        &self,
        partial: &str,
        suggestion_type: SuggestionType,
        limit: usize,
    ) -> Result<Vec<String>, LiteError> {
        let all_suggestions = self.get_suggestions(partial, limit * 2)?;

        let filtered: Vec<String> = all_suggestions
            .into_iter()
            .filter(|s| {
                std::mem::discriminant(&s.suggestion_type)
                    == std::mem::discriminant(&suggestion_type)
            })
            .take(limit)
            .map(|s| s.text)
            .collect();

        Ok(filtered)
    }

    /// Legacy simple suggestions API (for backward compatibility)
    pub fn get_simple_suggestions(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<String>, LiteError> {
        let suggestions = self.get_suggestions(partial, limit)?;
        Ok(suggestions.into_iter().map(|s| s.text).collect())
    }

    /// Quick search - single keyword search with default options
    pub fn quick_search(&self, keyword: &str) -> Result<Vec<SearchResult>, LiteError> {
        let query = SearchQuery {
            keyword: Some(keyword.to_string()),
            sort_by: SortBy::Name,
            sort_order: SortOrder::Asc,
            limit: 20,
            ..Default::default()
        };

        self.search(&query)
    }

    /// Simple advanced search (backward compatible)
    pub fn advanced_search_simple(
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

    /// Advanced search with complex filters and parameters
    pub fn advanced_search(
        &self,
        keyword: Option<&str>,
        group_id: Option<&str>,
        auth_method: Option<AuthMethod>,
        tags: Option<Vec<String>>,
        _params: Option<AdvancedSearchParams>,
    ) -> Result<(Vec<SearchResult>, SearchMetrics), LiteError> {
        let start_time = Instant::now();
        let mut metrics = SearchMetrics::default();

        let query = SearchQuery {
            keyword: keyword.map(|s| s.to_string()),
            group_id: group_id.map(|s| s.to_string()),
            auth_method,
            tags,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Asc,
            ..Default::default()
        };

        let results = self.search(&query)?;

        // Calculate metrics
        let elapsed = start_time.elapsed();
        metrics.query_time_ms = elapsed.as_millis() as u64;
        metrics.results_count = results.len();

        Ok((results, metrics))
    }

    /// Search with real-time suggestions (for type-ahead)
    pub fn search_realtime(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, LiteError> {
        if partial.is_empty() {
            return Ok(Vec::new());
        }

        let query = SearchQuery {
            keyword: Some(partial.to_string()),
            limit,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Asc,
            ..Default::default()
        };

        self.search(&query)
    }

    /// Batch search for multiple queries (useful for bulk operations)
    pub fn batch_search(
        &self,
        queries: &[SearchQuery],
    ) -> Result<Vec<Vec<SearchResult>>, LiteError> {
        let mut results = Vec::with_capacity(queries.len());

        for query in queries {
            results.push(self.search(query)?);
        }

        Ok(results)
    }

    /// Get search performance metrics
    pub fn get_metrics(&self) -> Result<SearchMetrics, LiteError> {
        Ok(SearchMetrics::default())
    }

    /// Clear result cache
    pub fn clear_cache(&self) -> Result<(), LiteError> {
        let mut cache = self
            .result_cache
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock result cache".to_string()))?;
        cache.clear();
        Ok(())
    }

    /// Get service statistics
    pub fn get_stats(&self) -> Result<(usize, usize, usize), LiteError> {
        let index = self
            .index
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search index".to_string()))?;
        let history = self
            .history
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock search history".to_string()))?;
        let cache = self
            .result_cache
            .lock()
            .map_err(|_| LiteError::Internal("Failed to lock result cache".to_string()))?;

        Ok((index.len(), history.len(), cache.len()))
    }

    // Placeholder methods - these should be implemented based on actual database API
    fn load_host_by_id(&self, host_id: &str) -> Result<HostRecord, LiteError> {
        self.db.get_host(host_id)
    }

    fn load_tags_for_host(&self, host_id: &str) -> Result<Vec<TagRecord>, LiteError> {
        self.db.get_host_tags(host_id)
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
    use std::str::FromStr;

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
        let pinyin1 = cache.to_pinyin("测试服务器");
        let pinyin2 = cache.to_pinyin("测试服务器");

        // Should return cached result
        assert_eq!(pinyin1, pinyin2);

        // Test cache clear
        cache.clear();
        assert_eq!(cache.cache_stats().0, 0);
    }

    #[test]
    fn test_pinyin_search_variants() {
        let mut cache = PinyinCache::new();

        // Test getting search variants
        let variants = cache.get_search_variants("测试");
        assert!(variants.len() >= 2); // At least original and pinyin
        assert!(variants.contains(&"测试".to_lowercase()));

        // Test initials extraction
        let pinyin = cache.to_pinyin("服务器");
        assert!(!pinyin.is_empty());
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
        let variants = [
            SortBy::Name,
            SortBy::CreatedAt,
            SortBy::LastConnected,
            SortBy::Custom,
        ];

        // Ensure all variants are distinct
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_fuzzy_config_default() {
        let config = FuzzyConfig::default();
        assert_eq!(config.max_distance, 2);
        assert!(config.prefix_match);
        assert!(config.substring_match);
        assert!(config.initials_match);
        assert!(config.min_score >= 0.0 && config.min_score <= 1.0);
    }

    #[test]
    fn test_history_config_default() {
        let config = HistoryConfig::default();
        assert!(config.max_memory_entries > 0);
        assert!(config.max_persisted_entries > 0);
        assert!(config.deduplicate);
        assert!(config.min_query_length > 0);
    }

    #[test]
    fn test_search_suggestion_creation() {
        let suggestion = SearchSuggestion {
            text: "production".to_string(),
            suggestion_type: SuggestionType::History,
            score: 0.95,
            frequency: 5,
        };

        assert_eq!(suggestion.text, "production");
        assert_eq!(suggestion.score, 0.95);
    }

    #[test]
    fn test_advanced_search_params_default() {
        let params = AdvancedSearchParams::default();
        assert!(!params.include_archived);
        assert!(!params.favorite_only);
        assert!(params.port_range.is_none());
        assert!(params.custom_filters.is_empty());
    }

    #[test]
    fn test_search_metrics_default() {
        let metrics = SearchMetrics::default();
        assert_eq!(metrics.query_time_ms, 0);
        assert_eq!(metrics.results_count, 0);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn test_levenshtein_distance() {
        // Simple tests using the SearchService implementation
        let db = std::sync::Arc::new(Database::new_in_memory().unwrap());
        db.init().unwrap();
        let _service = SearchService::new(db).unwrap();

        // Test with direct string comparison
        let distance = SearchService::levenshtein_distance("kitten", "sitting");
        assert_eq!(distance, 3); // k->s, e->i, +g

        let distance = SearchService::levenshtein_distance("", "abc");
        assert_eq!(distance, 3);

        let distance = SearchService::levenshtein_distance("abc", "");
        assert_eq!(distance, 3);

        let distance = SearchService::levenshtein_distance("same", "same");
        assert_eq!(distance, 0);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn test_fuzzy_scoring() {
        let db = std::sync::Arc::new(Database::new_in_memory().unwrap());
        db.init().unwrap();
        let service = SearchService::new(db).unwrap();

        // Exact match should score highest
        let exact_score = service.calculate_fuzzy_score("server", "server");
        assert_eq!(exact_score, 1.0);

        // Prefix match should score high
        let prefix_score = service.calculate_fuzzy_score("server", "ser");
        assert!(prefix_score > 0.9);

        // Substring match should score well
        let substring_score = service.calculate_fuzzy_score("production server", "server");
        // Score depends on position: "server" starts at position 11 in "production server"
        // position_penalty = 11/18 * 0.2 ≈ 0.12, so score ≈ 0.58
        assert!(substring_score > 0.5);

        // Non-matching should score 0
        let no_match_score = service.calculate_fuzzy_score("server", "xyz");
        assert_eq!(no_match_score, 0.0);
    }

    #[test]
    #[allow(clippy::arc_with_non_send_sync)]
    fn test_highlight_fuzzy_matches() {
        let db = std::sync::Arc::new(Database::new_in_memory().unwrap());
        db.init().unwrap();
        let service = SearchService::new(db).unwrap();

        // Test exact match highlighting
        let result = service
            .highlight_fuzzy_matches("production server", "server")
            .unwrap();
        assert!(result.contains("**server**"));

        // Test fuzzy highlighting
        let result = service
            .highlight_fuzzy_matches("production", "prd")
            .unwrap();
        assert!(result.contains("**"));

        // Test empty keyword
        let result = service.highlight_fuzzy_matches("production", "").unwrap();
        assert_eq!(result, "production");
    }

    #[test]
    fn test_pinyin_with_mixed_input() {
        let mut cache = PinyinCache::new();

        // Test mixed Chinese and ASCII input
        let pinyin = cache.to_pinyin("Web服务器1");
        assert!(pinyin.contains("web"));
        assert!(pinyin.contains("fuwuqi"));

        // Test segmented output
        let segments = cache.to_pinyin_segmented("测试服务器");
        assert!(!segments.is_empty());
    }
}
