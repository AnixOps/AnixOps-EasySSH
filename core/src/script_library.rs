use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use crate::workflow_engine::Workflow;
use crate::macro_recorder::Macro;

/// Script library manager - stores and manages reusable workflows and macros
#[derive(Debug, Clone)]
pub struct ScriptLibrary {
    /// Storage directory for scripts
    storage_path: PathBuf,
    /// In-memory cache of workflows
    workflows: HashMap<String, WorkflowEntry>,
    /// In-memory cache of macros
    macros: HashMap<String, MacroEntry>,
    /// Categories for organization
    categories: Vec<ScriptCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: String,
    pub workflow: Workflow,
    pub metadata: ScriptMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEntry {
    pub id: String,
    pub macro_data: Macro,
    pub metadata: ScriptMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub version: String,
    pub usage_count: u64,
    pub last_used: Option<DateTime<Utc>>,
    pub is_template: bool,
    pub is_favorite: bool,
    pub rating: Option<u8>, // 1-5
    pub tags: Vec<String>,
}

impl ScriptMetadata {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            created_by: None,
            version: "1.0.0".to_string(),
            usage_count: 0,
            last_used: None,
            is_template: false,
            is_favorite: false,
            rating: None,
            tags: Vec::new(),
        }
    }

    pub fn record_usage(&mut self) {
        self.usage_count += 1;
        self.last_used = Some(Utc::now());
    }
}

impl Default for ScriptMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<String>,
}

/// Search and filter options
#[derive(Debug, Clone, Default)]
pub struct ScriptSearchOptions {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub script_type: Option<ScriptType>,
    pub is_template: Option<bool>,
    pub is_favorite: Option<bool>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    Workflow,
    Macro,
    All,
}

impl ScriptLibrary {
    pub fn new(storage_path: PathBuf) -> Self {
        let mut lib = Self {
            storage_path,
            workflows: HashMap::new(),
            macros: HashMap::new(),
            categories: Self::default_categories(),
        };

        // Load existing scripts from disk
        lib.load_from_disk();
        lib
    }

    fn default_categories() -> Vec<ScriptCategory> {
        vec![
            ScriptCategory {
                id: "deployment".to_string(),
                name: "Deployment".to_string(),
                description: Some("Application deployment workflows".to_string()),
                icon: Some("rocket".to_string()),
                color: Some("#FF6B6B".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "maintenance".to_string(),
                name: "Maintenance".to_string(),
                description: Some("System maintenance and updates".to_string()),
                icon: Some("wrench".to_string()),
                color: Some("#4ECDC4".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "backup".to_string(),
                name: "Backup & Recovery".to_string(),
                description: Some("Data backup and disaster recovery".to_string()),
                icon: Some("archive".to_string()),
                color: Some("#45B7D1".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "monitoring".to_string(),
                name: "Monitoring".to_string(),
                description: Some("Health checks and monitoring".to_string()),
                icon: Some("activity".to_string()),
                color: Some("#96CEB4".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "security".to_string(),
                name: "Security".to_string(),
                description: Some("Security hardening and auditing".to_string()),
                icon: Some("shield".to_string()),
                color: Some("#FFEAA7".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "network".to_string(),
                name: "Network".to_string(),
                description: Some("Network configuration and diagnostics".to_string()),
                icon: Some("globe".to_string()),
                color: Some("#DDA0DD".to_string()),
                parent_id: None,
            },
            ScriptCategory {
                id: "custom".to_string(),
                name: "Custom".to_string(),
                description: Some("User-defined scripts".to_string()),
                icon: Some("file-code".to_string()),
                color: Some("#B0C4DE".to_string()),
                parent_id: None,
            },
        ]
    }

    /// Add a workflow to the library
    pub fn add_workflow(&mut self, workflow: Workflow, metadata: Option<ScriptMetadata>) -> String {
        let id = workflow.id.clone();
        let entry = WorkflowEntry {
            id: id.clone(),
            workflow,
            metadata: metadata.unwrap_or_default(),
        };

        self.workflows.insert(id.clone(), entry);
        self.save_to_disk();
        id
    }

    /// Add a macro to the library
    pub fn add_macro(&mut self, macro_data: Macro, metadata: Option<ScriptMetadata>) -> String {
        let id = macro_data.id.clone();
        let entry = MacroEntry {
            id: id.clone(),
            macro_data,
            metadata: metadata.unwrap_or_default(),
        };

        self.macros.insert(id.clone(), entry);
        self.save_to_disk();
        id
    }

    /// Get a workflow by ID
    pub fn get_workflow(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id).map(|e| &e.workflow)
    }

    /// Get a workflow entry (with metadata)
    pub fn get_workflow_entry(&self, id: &str) -> Option<&WorkflowEntry> {
        self.workflows.get(id)
    }

    /// Get a macro by ID
    pub fn get_macro(&self, id: &str) -> Option<&Macro> {
        self.macros.get(id).map(|e| &e.macro_data)
    }

    /// Get a macro entry (with metadata)
    pub fn get_macro_entry(&self, id: &str) -> Option<&MacroEntry> {
        self.macros.get(id)
    }

    /// Update a workflow
    pub fn update_workflow(&mut self, id: &str, workflow: Workflow) -> Result<(), String> {
        if let Some(entry) = self.workflows.get_mut(id) {
            entry.workflow = workflow;
            entry.metadata.updated_at = Utc::now();
            self.save_to_disk();
            Ok(())
        } else {
            Err(format!("Workflow {} not found", id))
        }
    }

    /// Update a macro
    pub fn update_macro(&mut self, id: &str, macro_data: Macro) -> Result<(), String> {
        if let Some(entry) = self.macros.get_mut(id) {
            entry.macro_data = macro_data;
            entry.metadata.updated_at = Utc::now();
            self.save_to_disk();
            Ok(())
        } else {
            Err(format!("Macro {} not found", id))
        }
    }

    /// Delete a workflow
    pub fn delete_workflow(&mut self, id: &str) -> Option<WorkflowEntry> {
        let entry = self.workflows.remove(id);
        if entry.is_some() {
            self.save_to_disk();
        }
        entry
    }

    /// Delete a macro
    pub fn delete_macro(&mut self, id: &str) -> Option<MacroEntry> {
        let entry = self.macros.remove(id);
        if entry.is_some() {
            self.save_to_disk();
        }
        entry
    }

    /// List all workflows
    pub fn list_workflows(&self) -> Vec<&WorkflowEntry> {
        self.workflows.values().collect()
    }

    /// List all macros
    pub fn list_macros(&self) -> Vec<&MacroEntry> {
        self.macros.values().collect()
    }

    /// Search scripts
    pub fn search(&self, options: ScriptSearchOptions) -> Vec<ScriptSearchResult<'_>> {
        let mut results = Vec::new();

        // Search workflows
        if options.script_type != Some(ScriptType::Macro) {
            for entry in self.workflows.values() {
                if self.matches_search(&entry.workflow.name, entry, &options) {
                    results.push(ScriptSearchResult::Workflow(entry));
                }
            }
        }

        // Search macros
        if options.script_type != Some(ScriptType::Workflow) {
            for entry in self.macros.values() {
                if self.matches_search(&entry.macro_data.name, entry, &options) {
                    results.push(ScriptSearchResult::Macro(entry));
                }
            }
        }

        results
    }

    fn matches_search(&self, name: &str, entry: &impl HasMetadata, options: &ScriptSearchOptions) -> bool {
        let metadata = entry.metadata();

        // Text search
        if let Some(ref query) = options.query {
            let query_lower = query.to_lowercase();
            if !name.to_lowercase().contains(&query_lower) {
                return false;
            }
        }

        // Category filter
        if let Some(ref _cat) = options.category {
            // Check if workflow/macro has matching category
            // This would check workflow.category or macro tags
        }

        // Tags filter
        if !options.tags.is_empty() {
            let has_tag = options.tags.iter()
                .any(|t| metadata.tags.contains(t));
            if !has_tag {
                return false;
            }
        }

        // Template filter
        if let Some(is_template) = options.is_template {
            if metadata.is_template != is_template {
                return false;
            }
        }

        // Favorite filter
        if let Some(is_fav) = options.is_favorite {
            if metadata.is_favorite != is_fav {
                return false;
            }
        }

        // Date filters
        if let Some(after) = options.created_after {
            if metadata.created_at < after {
                return false;
            }
        }
        if let Some(before) = options.created_before {
            if metadata.created_at > before {
                return false;
            }
        }

        true
    }

    /// Record that a script was used
    pub fn record_usage(&mut self, script_id: &str, script_type: ScriptType) {
        match script_type {
            ScriptType::Workflow => {
                if let Some(entry) = self.workflows.get_mut(script_id) {
                    entry.metadata.record_usage();
                }
            }
            ScriptType::Macro => {
                if let Some(entry) = self.macros.get_mut(script_id) {
                    entry.metadata.record_usage();
                }
            }
            _ => {}
        }
    }

    /// Set favorite status
    pub fn set_favorite(&mut self, script_id: &str, script_type: ScriptType, is_favorite: bool) {
        match script_type {
            ScriptType::Workflow => {
                if let Some(entry) = self.workflows.get_mut(script_id) {
                    entry.metadata.is_favorite = is_favorite;
                }
            }
            ScriptType::Macro => {
                if let Some(entry) = self.macros.get_mut(script_id) {
                    entry.metadata.is_favorite = is_favorite;
                }
            }
            _ => {}
        }
        self.save_to_disk();
    }

    /// Get favorite scripts
    pub fn get_favorites(&self) -> Vec<ScriptSearchResult<'_>> {
        let mut results = Vec::new();

        for entry in self.workflows.values() {
            if entry.metadata.is_favorite {
                results.push(ScriptSearchResult::Workflow(entry));
            }
        }

        for entry in self.macros.values() {
            if entry.metadata.is_favorite {
                results.push(ScriptSearchResult::Macro(entry));
            }
        }

        results
    }

    /// Get recently used scripts
    pub fn get_recently_used(&self, limit: usize) -> Vec<ScriptSearchResult<'_>> {
        let mut all: Vec<_> = self.list_workflows().into_iter()
            .map(|e| (e.metadata.last_used, ScriptSearchResult::Workflow(e)))
            .chain(self.list_macros().into_iter()
                .map(|e| (e.metadata.last_used, ScriptSearchResult::Macro(e))))
            .filter(|(date, _)| date.is_some())
            .collect();

        // Sort by last used (newest first)
        all.sort_by(|a, b| b.0.cmp(&a.0));

        all.into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect()
    }

    /// Get most used scripts
    pub fn get_most_used(&self, limit: usize) -> Vec<ScriptSearchResult<'_>> {
        let mut all: Vec<_> = self.list_workflows().into_iter()
            .map(|e| (e.metadata.usage_count, ScriptSearchResult::Workflow(e)))
            .chain(self.list_macros().into_iter()
                .map(|e| (e.metadata.usage_count, ScriptSearchResult::Macro(e))))
            .collect();

        // Sort by usage count (highest first)
        all.sort_by(|a, b| b.0.cmp(&a.0));

        all.into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect()
    }

    /// Export script to JSON
    pub fn export_to_json(&self, script_id: &str, script_type: ScriptType) -> Result<String, String> {
        match script_type {
            ScriptType::Workflow => {
                let entry = self.workflows.get(script_id)
                    .ok_or_else(|| format!("Workflow {} not found", script_id))?;
                serde_json::to_string_pretty(entry)
                    .map_err(|e| format!("Serialization error: {}", e))
            }
            ScriptType::Macro => {
                let entry = self.macros.get(script_id)
                    .ok_or_else(|| format!("Macro {} not found", script_id))?;
                serde_json::to_string_pretty(entry)
                    .map_err(|e| format!("Serialization error: {}", e))
            }
            _ => Err("Cannot export 'All' type".to_string()),
        }
    }

    /// Import script from JSON
    pub fn import_from_json(&mut self, json: &str, script_type: ScriptType) -> Result<String, String> {
        match script_type {
            ScriptType::Workflow => {
                let entry: WorkflowEntry = serde_json::from_str(json)
                    .map_err(|e| format!("Parse error: {}", e))?;
                let id = entry.id.clone();
                self.workflows.insert(id.clone(), entry);
                self.save_to_disk();
                Ok(id)
            }
            ScriptType::Macro => {
                let entry: MacroEntry = serde_json::from_str(json)
                    .map_err(|e| format!("Parse error: {}", e))?;
                let id = entry.id.clone();
                self.macros.insert(id.clone(), entry);
                self.save_to_disk();
                Ok(id)
            }
            _ => Err("Cannot import 'All' type".to_string()),
        }
    }

    /// Get or create category
    pub fn get_category(&self, id: &str) -> Option<&ScriptCategory> {
        self.categories.iter().find(|c| c.id == id)
    }

    /// Add custom category
    pub fn add_category(&mut self, category: ScriptCategory) {
        self.categories.push(category);
        self.save_to_disk();
    }

    /// Get all categories
    pub fn get_categories(&self) -> &[ScriptCategory] {
        &self.categories
    }

    fn load_from_disk(&mut self) {
        // Implementation would load from storage_path
        // For now, we'll just ensure the directory exists
        if !self.storage_path.exists() {
            let _ = std::fs::create_dir_all(&self.storage_path);
        }
    }

    fn save_to_disk(&self) {
        // Implementation would save to storage_path
        // This is a placeholder
    }
}

/// Trait for items that have metadata
trait HasMetadata {
    fn metadata(&self) -> &ScriptMetadata;
}

impl HasMetadata for WorkflowEntry {
    fn metadata(&self) -> &ScriptMetadata {
        &self.metadata
    }
}

impl HasMetadata for MacroEntry {
    fn metadata(&self) -> &ScriptMetadata {
        &self.metadata
    }
}

/// Search result enum
#[derive(Debug, Clone)]
pub enum ScriptSearchResult<'a> {
    Workflow(&'a WorkflowEntry),
    Macro(&'a MacroEntry),
}

impl<'a> ScriptSearchResult<'a> {
    pub fn id(&self) -> &str {
        match self {
            ScriptSearchResult::Workflow(e) => &e.id,
            ScriptSearchResult::Macro(e) => &e.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ScriptSearchResult::Workflow(e) => &e.workflow.name,
            ScriptSearchResult::Macro(e) => &e.macro_data.name,
        }
    }

    pub fn script_type(&self) -> ScriptType {
        match self {
            ScriptSearchResult::Workflow(_) => ScriptType::Workflow,
            ScriptSearchResult::Macro(_) => ScriptType::Macro,
        }
    }

    pub fn metadata(&self) -> &ScriptMetadata {
        match self {
            ScriptSearchResult::Workflow(e) => &e.metadata,
            ScriptSearchResult::Macro(e) => &e.metadata,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            ScriptSearchResult::Workflow(e) => e.workflow.description.as_deref(),
            ScriptSearchResult::Macro(e) => e.macro_data.description.as_deref(),
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            ScriptSearchResult::Workflow(e) => &e.workflow.tags,
            ScriptSearchResult::Macro(e) => &e.macro_data.tags,
        }
    }

    pub fn category(&self) -> Option<&str> {
        match self {
            ScriptSearchResult::Workflow(e) => e.workflow.category.as_deref(),
            ScriptSearchResult::Macro(_) => None,
        }
    }
}

/// Script bundle for sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBundle {
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub workflows: Vec<WorkflowEntry>,
    pub macros: Vec<MacroEntry>,
}

impl ScriptLibrary {
    /// Create a bundle of multiple scripts
    pub fn create_bundle(&self, workflow_ids: &[String], macro_ids: &[String]) -> ScriptBundle {
        let workflows: Vec<_> = workflow_ids.iter()
            .filter_map(|id| self.workflows.get(id).cloned())
            .collect();

        let macros: Vec<_> = macro_ids.iter()
            .filter_map(|id| self.macros.get(id).cloned())
            .collect();

        ScriptBundle {
            name: "Bundle".to_string(),
            description: None,
            created_at: Utc::now(),
            workflows,
            macros,
        }
    }

    /// Import a bundle
    pub fn import_bundle(&mut self, bundle: ScriptBundle) -> (Vec<String>, Vec<String>) {
        let mut workflow_ids = Vec::new();
        let mut macro_ids = Vec::new();

        for entry in bundle.workflows {
            let id = entry.id.clone();
            self.workflows.insert(id.clone(), entry);
            workflow_ids.push(id);
        }

        for entry in bundle.macros {
            let id = entry.id.clone();
            self.macros.insert(id.clone(), entry);
            macro_ids.push(id);
        }

        self.save_to_disk();
        (workflow_ids, macro_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_script_library() {
        let temp_dir = TempDir::new().unwrap();
        let lib = ScriptLibrary::new(temp_dir.path().to_path_buf());

        assert!(lib.list_workflows().is_empty());
        assert!(lib.list_macros().is_empty());
    }

    #[test]
    fn test_add_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut lib = ScriptLibrary::new(temp_dir.path().to_path_buf());

        let workflow = Workflow::new("Test Workflow");
        let id = lib.add_workflow(workflow, None);

        assert!(!id.is_empty());
        assert_eq!(lib.list_workflows().len(), 1);
    }

    #[test]
    fn test_search() {
        let temp_dir = TempDir::new().unwrap();
        let mut lib = ScriptLibrary::new(temp_dir.path().to_path_buf());

        let workflow = Workflow::new("Deployment Script");
        lib.add_workflow(workflow, None);

        let macro_data = Macro::new("Backup Macro");
        lib.add_macro(macro_data, None);

        let results = lib.search(ScriptSearchOptions {
            query: Some("deploy".to_string()),
            ..Default::default()
        });

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "Deployment Script");
    }
}
