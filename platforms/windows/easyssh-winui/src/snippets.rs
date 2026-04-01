#![allow(dead_code)]

//! Snippets System - Command Templates for EasySSH
//!
//! Provides reusable command templates with variable substitution,
//! categorization, and import/export functionality.
//!
//! Features:
//! - Categorized snippets (Frequently Used, Custom, Team)
//! - Template variables: {{hostname}}, {{username}}, {{port}}, {{password}}, {{custom}}
//! - Quick insert with parameter prompting
//! - Search and filter
//! - JSON import/export
//! - Team sharing support (Pro version)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Variable placeholder in snippet content
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnippetVariable {
    Hostname,
    Username,
    Port,
    Password,
    Custom {
        name: String,
        default: Option<String>,
    },
}

impl SnippetVariable {
    /// Parse variable from placeholder syntax {{name}}
    pub fn from_placeholder(placeholder: &str) -> Option<Self> {
        let name = placeholder
            .trim_matches(|c| c == '{' || c == '}')
            .to_lowercase();
        match name.as_str() {
            "hostname" | "host" => Some(Self::Hostname),
            "username" | "user" => Some(Self::Username),
            "port" => Some(Self::Port),
            "password" | "pass" | "pwd" => Some(Self::Password),
            custom => {
                // Check for custom variable with default: {{name|default_value}}
                if let Some(pipe_pos) = custom.find('|') {
                    let var_name = &custom[..pipe_pos];
                    let default = &custom[pipe_pos + 1..];
                    Some(Self::Custom {
                        name: var_name.to_string(),
                        default: Some(default.to_string()),
                    })
                } else {
                    Some(Self::Custom {
                        name: custom.to_string(),
                        default: None,
                    })
                }
            }
        }
    }

    /// Convert variable to placeholder string
    pub fn to_placeholder(&self) -> String {
        match self {
            Self::Hostname => "{{hostname}}".to_string(),
            Self::Username => "{{username}}".to_string(),
            Self::Port => "{{port}}".to_string(),
            Self::Password => "{{password}}".to_string(),
            Self::Custom {
                name,
                default: None,
            } => format!("{{{{{}}}}}", name),
            Self::Custom {
                name,
                default: Some(d),
            } => format!("{{{{{name}|{d}}}}}"),
        }
    }

    /// Get display name for the variable
    pub fn display_name(&self) -> String {
        match self {
            Self::Hostname => "Hostname".to_string(),
            Self::Username => "Username".to_string(),
            Self::Port => "Port".to_string(),
            Self::Password => "Password".to_string(),
            Self::Custom { name, .. } => name.clone(),
        }
    }
}

/// Snippet category for organization
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SnippetCategory {
    #[default]
    #[serde(rename = "frequently_used")]
    FrequentlyUsed,
    #[serde(rename = "custom")]
    Custom,
    #[serde(rename = "team")]
    Team,
    #[serde(rename = "system")]
    System,
}

impl SnippetCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FrequentlyUsed => "常用",
            Self::Custom => "自定义",
            Self::Team => "团队",
            Self::System => "系统",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::FrequentlyUsed => "⭐",
            Self::Custom => "🔧",
            Self::Team => "👥",
            Self::System => "⚙️",
        }
    }
}

/// Command snippet definition
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub category: SnippetCategory,
    pub tags: Vec<String>,
    pub shortcut: Option<String>, // Keyboard shortcut like "Ctrl+Shift+1"
    pub created_at: String,
    pub updated_at: String,
    pub is_shared: bool, // Team sharing flag
    pub author: Option<String>,
    pub usage_count: u32,
}

impl Snippet {
    /// Create a new snippet
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono::Local::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            content: content.into(),
            category: SnippetCategory::Custom,
            tags: Vec::new(),
            shortcut: None,
            created_at: now.clone(),
            updated_at: now,
            is_shared: false,
            author: None,
            usage_count: 0,
        }
    }

    /// Create a snippet with specific category
    pub fn with_category(mut self, category: SnippetCategory) -> Self {
        self.category = category;
        self
    }

    /// Create a snippet with description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Create a snippet with tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Create a snippet with shortcut
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Extract all variables from snippet content
    pub fn extract_variables(&self) -> Vec<SnippetVariable> {
        let mut variables = Vec::new();
        let mut found = std::collections::HashSet::new();

        // Find all {{variable}} patterns
        let mut start = 0;
        while let Some(pos) = self.content[start..].find("{{") {
            let actual_pos = start + pos;
            if let Some(end_pos) = self.content[actual_pos..].find("}}") {
                let placeholder = &self.content[actual_pos..actual_pos + end_pos + 2];
                if let Some(var) = SnippetVariable::from_placeholder(placeholder) {
                    let key = match &var {
                        SnippetVariable::Custom { name, .. } => name.clone(),
                        _ => format!("{:?}", var),
                    };
                    if !found.contains(&key) {
                        found.insert(key);
                        variables.push(var);
                    }
                }
                start = actual_pos + end_pos + 2;
            } else {
                break;
            }
        }

        variables
    }

    /// Render snippet with provided variable values
    pub fn render(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.content.clone();

        // Replace standard variables
        for (key, value) in values {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Replace custom variables with defaults if not provided
        for var in self.extract_variables() {
            if let SnippetVariable::Custom {
                name,
                default: Some(d),
            } = var
            {
                let placeholder = format!("{{{{{}}}}}", name);
                if !values.contains_key(&name) {
                    result = result.replace(&placeholder, &d);
                }
            }
        }

        result
    }

    /// Check if snippet requires user input (has unfilled variables)
    pub fn requires_input(&self) -> bool {
        !self.extract_variables().is_empty()
    }
}

/// Snippet collection for import/export
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SnippetCollection {
    pub version: String,
    pub export_date: String,
    pub snippets: Vec<Snippet>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Manager for snippets storage and operations
#[derive(Clone, Debug, Default)]
pub struct SnippetManager {
    snippets: Vec<Snippet>,
    search_query: String,
    selected_category: Option<SnippetCategory>,
}

impl SnippetManager {
    /// Create new snippet manager with default snippets
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.load_default_snippets();
        manager
    }

    /// Load built-in default snippets
    fn load_default_snippets(&mut self) {
        let defaults = vec![
            Snippet::new("List Files", "ls -la {{path|/home/{{username}}}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("List files in directory with details")
                .with_tags(vec!["files".to_string(), "ls".to_string()]),
            Snippet::new("Current Directory", "pwd")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Print working directory"),
            Snippet::new("Disk Usage", "df -h")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Show disk space usage"),
            Snippet::new("Memory Usage", "free -h")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Show memory usage"),
            Snippet::new("Process List", "ps aux | grep {{process_name|nginx}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Search processes by name"),
            Snippet::new("System Info", "uname -a && cat /etc/os-release")
                .with_category(SnippetCategory::System)
                .with_description("Show system information"),
            Snippet::new("Network Status", "ss -tuln")
                .with_category(SnippetCategory::System)
                .with_description("Show listening network ports"),
            Snippet::new("Docker PS", "docker ps -a")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("List all Docker containers")
                .with_tags(vec!["docker".to_string(), "containers".to_string()]),
            Snippet::new("Docker Logs", "docker logs -f {{container_name}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Follow Docker container logs")
                .with_tags(vec!["docker".to_string(), "logs".to_string()]),
            Snippet::new(
                "Find Large Files",
                "find {{path|/}} -type f -size +{{size|100M}} -exec ls -lh {} \\;",
            )
            .with_category(SnippetCategory::System)
            .with_description("Find files larger than specified size"),
            Snippet::new(
                "SSH to Server",
                "ssh {{username}}@{{hostname}} -p {{port|22}}",
            )
            .with_category(SnippetCategory::FrequentlyUsed)
            .with_description("SSH connection command")
            .with_tags(vec!["ssh".to_string(), "connect".to_string()]),
            Snippet::new("Tail Log", "tail -f {{log_path|/var/log/syslog}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Follow log file in real-time"),
            Snippet::new(
                "Grep Search",
                "grep -r \"{{search_term}}\" {{path|.}} --include=\"*.{{ext|txt}}\"",
            )
            .with_category(SnippetCategory::FrequentlyUsed)
            .with_description("Recursive grep search with file filter"),
            Snippet::new("Chmod Recursive", "chmod -R {{permissions|755}} {{path|.}}")
                .with_category(SnippetCategory::System)
                .with_description("Change permissions recursively"),
            Snippet::new(
                "Chown Recursive",
                "chown -R {{user|root}}:{{group|root}} {{path|.}}",
            )
            .with_category(SnippetCategory::System)
            .with_description("Change ownership recursively"),
            Snippet::new(
                "Create Tar Archive",
                "tar -czf {{archive_name|backup.tar.gz}} {{source_path|.}}",
            )
            .with_category(SnippetCategory::System)
            .with_description("Create compressed tar archive"),
            Snippet::new(
                "Extract Tar",
                "tar -xzf {{archive_path}} -C {{dest_path|.}}",
            )
            .with_category(SnippetCategory::System)
            .with_description("Extract tar archive"),
            Snippet::new("Git Status", "git status")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Check git repository status")
                .with_tags(vec!["git".to_string()]),
            Snippet::new("Git Pull", "git pull origin {{branch|main}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Pull latest changes from git remote")
                .with_tags(vec!["git".to_string()]),
            Snippet::new("Git Log", "git log --oneline -{{count|20}}")
                .with_category(SnippetCategory::FrequentlyUsed)
                .with_description("Show recent git commits")
                .with_tags(vec!["git".to_string()]),
            Snippet::new("Service Status", "systemctl status {{service_name}}")
                .with_category(SnippetCategory::System)
                .with_description("Check systemd service status"),
            Snippet::new("Service Restart", "sudo systemctl restart {{service_name}}")
                .with_category(SnippetCategory::System)
                .with_description("Restart systemd service"),
        ];

        self.snippets = defaults;
    }

    /// Get all snippets
    pub fn all_snippets(&self) -> &[Snippet] {
        &self.snippets
    }

    /// Get filtered snippets based on search and category
    pub fn filtered_snippets(&self) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|s| {
                // Category filter
                if let Some(ref cat) = self.selected_category {
                    if s.category != *cat {
                        return false;
                    }
                }

                // Search filter
                if self.search_query.is_empty() {
                    return true;
                }

                let query = self.search_query.to_lowercase();
                s.name.to_lowercase().contains(&query)
                    || s.content.to_lowercase().contains(&query)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    || s.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Add a new snippet
    pub fn add_snippet(&mut self, snippet: Snippet) {
        self.snippets.push(snippet);
    }

    /// Delete a snippet by ID
    pub fn delete_snippet(&mut self, id: &str) -> bool {
        let idx = self.snippets.iter().position(|s| s.id == id);
        if let Some(i) = idx {
            self.snippets.remove(i);
            true
        } else {
            false
        }
    }

    /// Get snippet by ID
    pub fn get_snippet(&self, id: &str) -> Option<&Snippet> {
        self.snippets.iter().find(|s| s.id == id)
    }

    /// Get mutable snippet by ID
    pub fn get_snippet_mut(&mut self, id: &str) -> Option<&mut Snippet> {
        self.snippets.iter_mut().find(|s| s.id == id)
    }

    /// Update snippet usage count
    pub fn record_usage(&mut self, id: &str) {
        if let Some(s) = self.get_snippet_mut(id) {
            s.usage_count += 1;
        }
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Get current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Set selected category filter
    pub fn set_category(&mut self, category: Option<SnippetCategory>) {
        self.selected_category = category;
    }

    /// Get selected category
    pub fn selected_category(&self) -> Option<&SnippetCategory> {
        self.selected_category.as_ref()
    }

    /// Get snippets by category
    pub fn snippets_by_category(&self, category: &SnippetCategory) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|s| &s.category == category)
            .collect()
    }

    /// Export snippets to JSON
    pub fn export_to_json(&self) -> anyhow::Result<String> {
        let collection = SnippetCollection {
            version: "1.0".to_string(),
            export_date: chrono::Local::now().to_rfc3339(),
            snippets: self.snippets.clone(),
            metadata: None,
        };
        Ok(serde_json::to_string_pretty(&collection)?)
    }

    /// Export specific category to JSON
    pub fn export_category_to_json(&self, category: &SnippetCategory) -> anyhow::Result<String> {
        let filtered: Vec<Snippet> = self
            .snippets
            .iter()
            .filter(|s| &s.category == category)
            .cloned()
            .collect();

        let collection = SnippetCollection {
            version: "1.0".to_string(),
            export_date: chrono::Local::now().to_rfc3339(),
            snippets: filtered,
            metadata: Some({
                let mut m = HashMap::new();
                m.insert("category".to_string(), format!("{:?}", category));
                m
            }),
        };
        Ok(serde_json::to_string_pretty(&collection)?)
    }

    /// Import snippets from JSON
    pub fn import_from_json(&mut self, json: &str) -> anyhow::Result<usize> {
        let collection: SnippetCollection = serde_json::from_str(json)?;
        let count = collection.snippets.len();

        // Generate new IDs for imported snippets to avoid conflicts
        for mut snippet in collection.snippets {
            snippet.id = uuid::Uuid::new_v4().to_string();
            snippet.created_at = chrono::Local::now().to_rfc3339();
            snippet.updated_at = chrono::Local::now().to_rfc3339();
            snippet.usage_count = 0;
            self.snippets.push(snippet);
        }

        Ok(count)
    }

    /// Merge imported snippets (with deduplication by name)
    pub fn merge_import(&mut self, json: &str) -> anyhow::Result<(usize, usize)> {
        let collection: SnippetCollection = serde_json::from_str(json)?;

        let mut added = 0;
        let mut skipped = 0;

        for snippet in collection.snippets {
            // Check for duplicate by name
            let exists = self
                .snippets
                .iter()
                .any(|s| s.name == snippet.name && s.content == snippet.content);

            if exists {
                skipped += 1;
            } else {
                let mut new_snippet = snippet;
                new_snippet.id = uuid::Uuid::new_v4().to_string();
                new_snippet.created_at = chrono::Local::now().to_rfc3339();
                new_snippet.updated_at = chrono::Local::now().to_rfc3339();
                new_snippet.usage_count = 0;
                self.snippets.push(new_snippet);
                added += 1;
            }
        }

        Ok((added, skipped))
    }

    /// Get frequently used snippets (sorted by usage)
    pub fn frequently_used(&self, limit: usize) -> Vec<&Snippet> {
        let mut sorted: Vec<&Snippet> = self.snippets.iter().collect();
        sorted.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        sorted.into_iter().take(limit).collect()
    }

    /// Find snippets by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Snippet> {
        self.snippets
            .iter()
            .filter(|s| s.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect()
    }

    /// Find snippet by keyboard shortcut
    pub fn find_by_shortcut(&self, shortcut: &str) -> Option<&Snippet> {
        self.snippets
            .iter()
            .find(|s| s.shortcut.as_deref() == Some(shortcut))
    }
}

/// UI state for snippet variable input dialog
#[derive(Clone, Debug, Default)]
pub struct SnippetInputDialog {
    pub snippet_id: Option<String>,
    pub snippet_name: String,
    pub snippet_content: String,
    pub variables: Vec<SnippetVariable>,
    pub values: HashMap<String, String>,
    pub current_variable_idx: usize,
    pub visible: bool,
}

impl SnippetInputDialog {
    pub fn new(snippet: &Snippet) -> Self {
        let variables = snippet.extract_variables();
        Self {
            snippet_id: Some(snippet.id.clone()),
            snippet_name: snippet.name.clone(),
            snippet_content: snippet.content.clone(),
            variables,
            values: HashMap::new(),
            current_variable_idx: 0,
            visible: true,
        }
    }

    /// Create from individual fields (avoids borrow issues in UI)
    pub fn from_fields(name: &str, content: &str) -> Self {
        let variables = Self::extract_variables_from_content(content);
        Self {
            snippet_id: None,
            snippet_name: name.to_string(),
            snippet_content: content.to_string(),
            variables,
            values: HashMap::new(),
            current_variable_idx: 0,
            visible: true,
        }
    }

    /// Extract variables from content string
    fn extract_variables_from_content(content: &str) -> Vec<SnippetVariable> {
        let mut variables = Vec::new();
        let mut found = std::collections::HashSet::new();
        let mut start = 0;
        while let Some(pos) = content[start..].find("{{") {
            let actual_pos = start + pos;
            if let Some(end_pos) = content[actual_pos..].find("}}") {
                let placeholder = &content[actual_pos..actual_pos + end_pos + 2];
                if let Some(var) = SnippetVariable::from_placeholder(placeholder) {
                    let key = match &var {
                        SnippetVariable::Custom { name, .. } => name.clone(),
                        _ => format!("{:?}", var),
                    };
                    if !found.contains(&key) {
                        found.insert(key);
                        variables.push(var);
                    }
                }
                start = actual_pos + end_pos + 2;
            } else {
                break;
            }
        }
        variables
    }

    /// Get the current variable being edited
    pub fn current_variable(&self) -> Option<&SnippetVariable> {
        self.variables.get(self.current_variable_idx)
    }

    /// Set value for current variable and advance
    pub fn set_current_value(&mut self, value: String) -> bool {
        if let Some(var) = self.current_variable() {
            let key = match var {
                SnippetVariable::Hostname => "hostname".to_string(),
                SnippetVariable::Username => "username".to_string(),
                SnippetVariable::Port => "port".to_string(),
                SnippetVariable::Password => "password".to_string(),
                SnippetVariable::Custom { name, .. } => name.clone(),
            };
            self.values.insert(key, value);

            // Check if we're done
            if self.current_variable_idx + 1 >= self.variables.len() {
                true // All variables filled
            } else {
                self.current_variable_idx += 1;
                false // More variables to fill
            }
        } else {
            true // No variables
        }
    }

    /// Get default value for current variable
    pub fn current_default(&self) -> Option<String> {
        self.current_variable().and_then(|v| match v {
            SnippetVariable::Custom {
                default: Some(d), ..
            } => Some(d.clone()),
            _ => None,
        })
    }

    /// Get current variable display name
    pub fn current_display_name(&self) -> String {
        self.current_variable()
            .map(|v| v.display_name())
            .unwrap_or_else(|| "Value".to_string())
    }

    /// Render the final command with all values filled in
    pub fn render_command(&self) -> String {
        let mut result = self.snippet_content.clone();
        for (key, value) in &self.values {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Replace remaining with defaults
        for var in &self.variables {
            if let SnippetVariable::Custom {
                name,
                default: Some(d),
            } = var
            {
                let placeholder = format!("{{{{{}}}}}", name);
                if !self.values.contains_key(name) {
                    result = result.replace(&placeholder, d);
                }
            }
        }

        result
    }

    /// Reset dialog state
    pub fn reset(&mut self) {
        self.snippet_id = None;
        self.snippet_name.clear();
        self.snippet_content.clear();
        self.variables.clear();
        self.values.clear();
        self.current_variable_idx = 0;
        self.visible = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_creation() {
        let snippet = Snippet::new("Test", "echo {{hostname}}");
        assert_eq!(snippet.name, "Test");
        assert_eq!(snippet.content, "echo {{hostname}}");
        assert!(!snippet.id.is_empty());
    }

    #[test]
    fn test_variable_parsing() {
        let var = SnippetVariable::from_placeholder("{{hostname}}");
        assert!(matches!(var, Some(SnippetVariable::Hostname)));

        let var = SnippetVariable::from_placeholder("{{custom_var|default_value}}");
        assert!(
            matches!(var, Some(SnippetVariable::Custom { name, default: Some(d) }) if name == "custom_var" && d == "default_value")
        );
    }

    #[test]
    fn test_extract_variables() {
        let snippet = Snippet::new("Test", "ssh {{username}}@{{hostname}} -p {{port|22}}");
        let vars = snippet.extract_variables();
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn test_render_snippet() {
        let snippet = Snippet::new("Test", "ssh {{username}}@{{hostname}}");
        let mut values = HashMap::new();
        values.insert("username".to_string(), "admin".to_string());
        values.insert("hostname".to_string(), "server.com".to_string());

        let result = snippet.render(&values);
        assert_eq!(result, "ssh admin@server.com");
    }

    #[test]
    fn test_snippet_manager() {
        let mut manager = SnippetManager::new();
        assert!(!manager.all_snippets().is_empty());

        let initial_count = manager.all_snippets().len();
        let snippet = Snippet::new("Custom", "echo test");
        manager.add_snippet(snippet);
        assert_eq!(manager.all_snippets().len(), initial_count + 1);
    }

    #[test]
    fn test_search_filter() {
        let mut manager = SnippetManager::new();
        manager.set_search_query("docker".to_string());
        let results = manager.filtered_snippets();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .all(|s| s.name.to_lowercase().contains("docker")
                || s.content.to_lowercase().contains("docker")));
    }

    #[test]
    fn test_category_filter() {
        let mut manager = SnippetManager::new();
        manager.set_category(Some(SnippetCategory::System));
        let results = manager.filtered_snippets();
        assert!(results
            .iter()
            .all(|s| s.category == SnippetCategory::System));
    }

    #[test]
    fn test_export_import() {
        let mut manager = SnippetManager::new();

        // Export
        let json = manager.export_to_json().unwrap();
        assert!(!json.is_empty());

        // Clear and import
        let count_before = manager.all_snippets().len();
        let imported = manager.import_from_json(&json).unwrap();
        assert_eq!(imported, count_before);
    }
}
