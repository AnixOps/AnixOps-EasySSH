#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::viewmodels::ServerViewModel;

/// Search result type
#[derive(Clone, Debug, PartialEq)]
pub enum SearchResultType {
    Server,
    CommandHistory,
    Snippet,
    Tag,
    Group,
}

/// Search result item
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub result_type: SearchResultType,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f32,
    pub action: QuickAction,
    pub metadata: HashMap<String, String>,
}

/// Quick action for search results
#[derive(Clone, Debug, PartialEq)]
pub enum QuickAction {
    Connect,
    Edit,
    Delete,
    Execute,
    FilterByTag,
    FilterByGroup,
    CopyToClipboard,
}

/// Fuzzy match score
#[derive(Clone, Debug)]
struct MatchScore {
    exact_match: bool,
    prefix_match: bool,
    contains_match: bool,
    fuzzy_score: f32,
    pinyin_match: bool,
}

impl MatchScore {
    fn total_score(&self) -> f32 {
        let mut score = self.fuzzy_score;
        if self.exact_match {
            score += 100.0;
        }
        if self.prefix_match {
            score += 50.0;
        }
        if self.contains_match {
            score += 20.0;
        }
        if self.pinyin_match {
            score += 15.0;
        }
        score
    }
}

/// Search history entry
#[derive(Clone, Debug)]
pub struct SearchHistoryEntry {
    pub query: String,
    pub timestamp: std::time::SystemTime,
    pub result_selected: Option<String>,
}

/// Filter criteria
#[derive(Clone, Debug, Default)]
pub struct FilterCriteria {
    pub tags: Vec<String>,
    pub group_id: Option<String>,
    pub connection_status: Option<ConnectionStatusFilter>,
    pub os_type: Option<String>,
    pub only_favorites: bool,
    pub only_recent: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionStatusFilter {
    Connected,
    Disconnected,
    All,
}

/// Recent usage tracker
#[derive(Clone, Debug)]
pub struct RecentUsageTracker {
    pub connections: Vec<RecentConnection>,
    pub commands: Vec<RecentCommand>,
    max_recent: usize,
}

#[derive(Clone, Debug)]
pub struct RecentConnection {
    pub server_id: String,
    pub server_name: String,
    pub timestamp: std::time::SystemTime,
    pub duration_secs: u64,
}

#[derive(Clone, Debug)]
pub struct RecentCommand {
    pub command: String,
    pub server_id: Option<String>,
    pub timestamp: std::time::SystemTime,
}

impl RecentUsageTracker {
    pub fn new(max_recent: usize) -> Self {
        Self {
            connections: Vec::new(),
            commands: Vec::new(),
            max_recent,
        }
    }

    pub fn record_connection(&mut self, server_id: String, server_name: String) {
        // Remove existing entry for this server
        self.connections.retain(|c| c.server_id != server_id);

        self.connections.push(RecentConnection {
            server_id,
            server_name,
            timestamp: std::time::SystemTime::now(),
            duration_secs: 0,
        });

        // Keep only max_recent
        if self.connections.len() > self.max_recent {
            self.connections.remove(0);
        }
    }

    pub fn update_connection_duration(&mut self, server_id: &str, duration_secs: u64) {
        if let Some(conn) = self.connections.iter_mut().find(|c| c.server_id == server_id) {
            conn.duration_secs = duration_secs;
        }
    }

    pub fn record_command(&mut self, command: String, server_id: Option<String>) {
        self.commands.push(RecentCommand {
            command,
            server_id,
            timestamp: std::time::SystemTime::now(),
        });

        if self.commands.len() > self.max_recent {
            self.commands.remove(0);
        }
    }

    pub fn get_recent_servers(&self, limit: usize) -> Vec<&RecentConnection> {
        self.connections.iter().rev().take(limit).collect()
    }

    pub fn get_recent_commands(&self, limit: usize) -> Vec<&RecentCommand> {
        self.commands.iter().rev().take(limit).collect()
    }
}

/// Pinyin converter for Chinese fuzzy search
pub struct PinyinConverter;

impl PinyinConverter {
    /// Convert Chinese characters to pinyin (simplified implementation)
    /// In production, use a proper pinyin library like `pinyin` crate
    pub fn to_pinyin(text: &str) -> String {
        // Simplified: just lowercase for now
        // Real implementation would convert Chinese to pinyin
        text.to_lowercase()
    }

    /// Check if text matches pinyin query
    pub fn matches_pinyin(text: &str, query: &str) -> bool {
        let pinyin = Self::to_pinyin(text);
        let query_lower = query.to_lowercase();

        pinyin.contains(&query_lower) ||
        Self::fuzzy_match(&pinyin, &query_lower)
    }

    /// Simple fuzzy matching algorithm
    fn fuzzy_match(text: &str, pattern: &str) -> bool {
        if pattern.is_empty() {
            return true;
        }

        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        let mut text_idx = 0;
        let mut pattern_idx = 0;

        while text_idx < text_chars.len() && pattern_idx < pattern_chars.len() {
            if text_chars[text_idx].to_lowercase().next() == pattern_chars[pattern_idx].to_lowercase().next() {
                pattern_idx += 1;
            }
            text_idx += 1;
        }

        pattern_idx == pattern_chars.len()
    }
}

/// Global search engine
pub struct GlobalSearchEngine {
    search_history: Vec<SearchHistoryEntry>,
    max_history: usize,
    snippets: Vec<Snippet>,
    recent_usage: RecentUsageTracker,
}

#[derive(Clone, Debug)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: std::time::SystemTime,
}

impl GlobalSearchEngine {
    pub fn new() -> Self {
        Self {
            search_history: Vec::new(),
            max_history: 50,
            snippets: Self::default_snippets(),
            recent_usage: RecentUsageTracker::new(20),
        }
    }

    fn default_snippets() -> Vec<Snippet> {
        vec![
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "Check disk usage".to_string(),
                content: "df -h".to_string(),
                tags: vec!["system".to_string(), "disk".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "Memory usage".to_string(),
                content: "free -h".to_string(),
                tags: vec!["system".to_string(), "memory".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "CPU info".to_string(),
                content: "lscpu".to_string(),
                tags: vec!["system".to_string(), "cpu".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "List processes".to_string(),
                content: "ps aux --sort=-%cpu | head -20".to_string(),
                tags: vec!["process".to_string(), "performance".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "Network connections".to_string(),
                content: "ss -tuln".to_string(),
                tags: vec!["network".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "Docker containers".to_string(),
                content: "docker ps -a".to_string(),
                tags: vec!["docker".to_string(), "containers".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "System updates".to_string(),
                content: "sudo apt update && sudo apt upgrade -y".to_string(),
                tags: vec!["update".to_string(), "ubuntu".to_string()],
                created_at: std::time::SystemTime::now(),
            },
            Snippet {
                id: Uuid::new_v4().to_string(),
                title: "Find large files".to_string(),
                content: r"find / -type f -size +100M -exec ls -lh {} \; 2>/dev/null".to_string(),
                tags: vec!["files".to_string(), "cleanup".to_string()],
                created_at: std::time::SystemTime::now(),
            },
        ]
    }

    /// Perform fuzzy search across all data sources
    pub fn search(
        &self,
        query: &str,
        servers: &[ServerViewModel],
        favorites: &HashSet<String>,
        command_history: &[String],
        tags: &HashMap<String, Vec<String>>,
        filter: &FilterCriteria,
        active_sessions: &[String], // server IDs with active connections
    ) -> Vec<SearchResult> {
        if query.is_empty() && !filter.only_favorites && !filter.only_recent {
            return self.get_default_results(servers, favorites, active_sessions);
        }

        let mut results: Vec<SearchResult> = Vec::new();
        let query_lower = query.to_lowercase();

        // Search servers
        for server in servers {
            // Apply filters
            if !self.matches_filters(server, filter, favorites, tags, active_sessions) {
                continue;
            }

            let score = self.calculate_match_score(&server.name, &query_lower);
            let host_score = self.calculate_match_score(&server.host, &query_lower);
            let user_score = self.calculate_match_score(&server.username, &query_lower);

            let best_score = score.total_score().max(host_score.total_score()).max(user_score.total_score());

            if query.is_empty() || best_score > 0.0 {
                let is_fav = favorites.contains(&server.id);
                let is_connected = active_sessions.contains(&server.id);

                // Boost favorites
                let final_score = if is_fav { best_score + 1000.0 } else { best_score };

                // Boost connected servers
                let final_score = if is_connected { final_score + 500.0 } else { final_score };

                // Get server tags
                let server_tags = tags.get(&server.id)
                    .map(|t| t.join(", "))
                    .unwrap_or_default();

                let subtitle = if is_connected {
                    format!("{}@{}:{} ● Connected", server.username, server.host, server.port)
                } else {
                    format!("{}@{}:{}", server.username, server.host, server.port)
                };

                results.push(SearchResult {
                    id: server.id.clone(),
                    result_type: SearchResultType::Server,
                    title: format!("{}{}", if is_fav { "★ " } else { "" }, server.name),
                    subtitle,
                    icon: if is_connected { "●" } else { "🖥" }.to_string(),
                    score: final_score,
                    action: QuickAction::Connect,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("host".to_string(), server.host.clone());
                        m.insert("username".to_string(), server.username.clone());
                        m.insert("port".to_string(), server.port.to_string());
                        m.insert("is_favorite".to_string(), is_fav.to_string());
                        m.insert("is_connected".to_string(), is_connected.to_string());
                        if !server_tags.is_empty() {
                            m.insert("tags".to_string(), server_tags);
                        }
                        m
                    },
                });
            }
        }

        // Search command history
        for cmd in command_history.iter().rev().take(20) {
            let score = self.calculate_match_score(cmd, &query_lower);
            if query.is_empty() || score.total_score() > 0.0 {
                results.push(SearchResult {
                    id: format!("cmd_{}", Uuid::new_v4()),
                    result_type: SearchResultType::CommandHistory,
                    title: cmd.clone(),
                    subtitle: "Command History".to_string(),
                    icon: "⌨".to_string(),
                    score: score.total_score() * 0.8, // Lower priority than servers
                    action: QuickAction::Execute,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("command".to_string(), cmd.clone());
                        m
                    },
                });
            }
        }

        // Search snippets
        for snippet in &self.snippets {
            let title_score = self.calculate_match_score(&snippet.title, &query_lower);
            let content_score = self.calculate_match_score(&snippet.content, &query_lower);
            let best_score = title_score.total_score().max(content_score.total_score());

            if query.is_empty() || best_score > 0.0 {
                results.push(SearchResult {
                    id: snippet.id.clone(),
                    result_type: SearchResultType::Snippet,
                    title: snippet.title.clone(),
                    subtitle: snippet.content.clone(),
                    icon: "📋".to_string(),
                    score: best_score * 0.9,
                    action: QuickAction::Execute,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("content".to_string(), snippet.content.clone());
                        m.insert("tags".to_string(), snippet.tags.join(", "));
                        m
                    },
                });
            }
        }

        // Search tags for filtering
        let all_tags: HashSet<String> = tags.values().flatten().cloned().collect();
        for tag in all_tags {
            let score = self.calculate_match_score(&tag, &query_lower);
            if score.total_score() > 0.0 || query.is_empty() {
                results.push(SearchResult {
                    id: format!("tag_{}", tag),
                    result_type: SearchResultType::Tag,
                    title: format!("#{}", tag),
                    subtitle: "Filter by tag".to_string(),
                    icon: "🏷".to_string(),
                    score: score.total_score() * 0.6,
                    action: QuickAction::FilterByTag,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("tag".to_string(), tag.clone());
                        m
                    },
                });
            }
        }

        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        results
    }

    /// Calculate fuzzy match score
    fn calculate_match_score(&self, text: &str, query: &str) -> MatchScore {
        if query.is_empty() {
            return MatchScore {
                exact_match: false,
                prefix_match: false,
                contains_match: false,
                fuzzy_score: 0.0,
                pinyin_match: false,
            };
        }

        let text_lower = text.to_lowercase();

        let exact_match = text_lower == query;
        let prefix_match = text_lower.starts_with(query);
        let contains_match = text_lower.contains(query);

        let fuzzy_score = if exact_match {
            100.0
        } else if prefix_match {
            80.0 - (text.len() as f32 - query.len() as f32) * 0.5
        } else if contains_match {
            60.0 - (text.len() as f32 - query.len() as f32) * 0.3
        } else {
            // Fuzzy match score
            if PinyinConverter::fuzzy_match(text, query) {
                40.0
            } else {
                0.0
            }
        };

        let pinyin_match = PinyinConverter::matches_pinyin(text, query);

        MatchScore {
            exact_match,
            prefix_match,
            contains_match,
            fuzzy_score: fuzzy_score.max(0.0),
            pinyin_match,
        }
    }

    /// Check if server matches filter criteria
    fn matches_filters(
        &self,
        server: &ServerViewModel,
        filter: &FilterCriteria,
        favorites: &HashSet<String>,
        tags: &HashMap<String, Vec<String>>,
        active_sessions: &[String],
    ) -> bool {
        // Only favorites filter
        if filter.only_favorites && !favorites.contains(&server.id) {
            return false;
        }

        // Only recent filter
        if filter.only_recent {
            let is_recent = self.recent_usage.connections.iter().any(|c| c.server_id == server.id);
            if !is_recent {
                return false;
            }
        }

        // Group filter
        if let Some(ref group_id) = filter.group_id {
            if server.group_id.as_ref() != Some(group_id) {
                return false;
            }
        }

        // Tags filter
        if !filter.tags.is_empty() {
            let server_tags = tags.get(&server.id).map(|t| t.as_slice()).unwrap_or(&[]);
            if !filter.tags.iter().all(|tag| server_tags.contains(tag)) {
                return false;
            }
        }

        // Connection status filter
        if let Some(ref status) = filter.connection_status {
            let is_connected = active_sessions.contains(&server.id);
            match status {
                ConnectionStatusFilter::Connected if !is_connected => return false,
                ConnectionStatusFilter::Disconnected if is_connected => return false,
                _ => {}
            }
        }

        true
    }

    /// Get default results when query is empty
    fn get_default_results(
        &self,
        servers: &[ServerViewModel],
        favorites: &HashSet<String>,
        active_sessions: &[String],
    ) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // Recent connections first
        for recent in self.recent_usage.get_recent_servers(5) {
            if let Some(server) = servers.iter().find(|s| s.id == recent.server_id) {
                let is_fav = favorites.contains(&server.id);
                let is_connected = active_sessions.contains(&server.id);

                results.push(SearchResult {
                    id: server.id.clone(),
                    result_type: SearchResultType::Server,
                    title: format!("{}Recent: {}{}", if is_fav { "★ " } else { "" }, server.name, if is_connected { " ●" } else { "" }),
                    subtitle: format!("{}@{}:{}", server.username, server.host, server.port),
                    icon: "🕐".to_string(),
                    score: 2000.0,
                    action: QuickAction::Connect,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("host".to_string(), server.host.clone());
                        m.insert("is_recent".to_string(), "true".to_string());
                        m
                    },
                });
            }
        }

        // Active sessions
        for session_id in active_sessions {
            if let Some(server) = servers.iter().find(|s| &s.id == session_id) {
                results.push(SearchResult {
                    id: server.id.clone(),
                    result_type: SearchResultType::Server,
                    title: format!("● Active: {}", server.name),
                    subtitle: format!("{}@{}:{}", server.username, server.host, server.port),
                    icon: "●".to_string(),
                    score: 1900.0,
                    action: QuickAction::Connect,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("host".to_string(), server.host.clone());
                        m.insert("is_connected".to_string(), "true".to_string());
                        m
                    },
                });
            }
        }

        // Favorite servers
        for server in servers {
            if favorites.contains(&server.id) && !results.iter().any(|r| r.id == server.id) {
                results.push(SearchResult {
                    id: server.id.clone(),
                    result_type: SearchResultType::Server,
                    title: format!("★ {}", server.name),
                    subtitle: format!("{}@{}:{}", server.username, server.host, server.port),
                    icon: "★".to_string(),
                    score: 1800.0,
                    action: QuickAction::Connect,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("host".to_string(), server.host.clone());
                        m.insert("is_favorite".to_string(), "true".to_string());
                        m
                    },
                });
            }
        }

        // Popular snippets
        for (idx, snippet) in self.snippets.iter().take(3).enumerate() {
            results.push(SearchResult {
                id: snippet.id.clone(),
                result_type: SearchResultType::Snippet,
                title: snippet.title.clone(),
                subtitle: snippet.content.clone(),
                icon: "📋".to_string(),
                score: 1000.0 - (idx as f32 * 100.0),
                action: QuickAction::Execute,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("content".to_string(), snippet.content.clone());
                    m
                },
            });
        }

        results
    }

    /// Add search to history
    pub fn add_to_history(&mut self, query: &str, result_selected: Option<String>) {
        if query.is_empty() {
            return;
        }

        // Remove existing entry with same query
        self.search_history.retain(|h| h.query != query);

        self.search_history.push(SearchHistoryEntry {
            query: query.to_string(),
            timestamp: std::time::SystemTime::now(),
            result_selected,
        });

        if self.search_history.len() > self.max_history {
            self.search_history.remove(0);
        }
    }

    /// Get search history
    pub fn get_search_history(&self) -> &[SearchHistoryEntry] {
        &self.search_history
    }

    /// Get recent usage tracker
    pub fn recent_usage(&self) -> &RecentUsageTracker {
        &self.recent_usage
    }

    pub fn recent_usage_mut(&mut self) -> &mut RecentUsageTracker {
        &mut self.recent_usage
    }

    /// Add new snippet
    pub fn add_snippet(&mut self, title: String, content: String, tags: Vec<String>) {
        self.snippets.push(Snippet {
            id: Uuid::new_v4().to_string(),
            title,
            content,
            tags,
            created_at: std::time::SystemTime::now(),
        });
    }

    /// Get all snippets
    pub fn get_snippets(&self) -> &[Snippet] {
        &self.snippets
    }

    /// Clear search history
    pub fn clear_history(&mut self) {
        self.search_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(PinyinConverter::fuzzy_match("production", "prod"));
        assert!(PinyinConverter::fuzzy_match("Production Server", "ps"));
        assert!(PinyinConverter::fuzzy_match("API Gateway", "apig"));
        assert!(!PinyinConverter::fuzzy_match("test", "xyz"));
    }

    #[test]
    fn test_match_score() {
        let engine = GlobalSearchEngine::new();

        let score1 = engine.calculate_match_score("Production", "prod");
        assert!(score1.prefix_match);

        let score2 = engine.calculate_match_score("Production", "ction");
        assert!(score2.contains_match);

        let score3 = engine.calculate_match_score("Production", "production");
        assert!(score3.exact_match);
    }
}
