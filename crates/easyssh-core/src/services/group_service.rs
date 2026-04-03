//! Group Service
//!
//! Provides comprehensive group management functionality including:
//! - CRUD operations for server groups
//! - Server-to-group assignment
//! - Group statistics and analytics
//! - Batch operations
//! - Import/export support
//! - Transaction support for data integrity

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{Database, GroupRecord, NewGroup, UpdateGroup};
use crate::error::{CoreDatabaseError, EasySSHErrors, LiteError};
use crate::models::group::{
    CreateGroupRequest, Group, GroupId, GroupStats, GroupWithServers, MoveServerRequest,
    ServerReference, UpdateGroupRequest, UNGROUPED_COLOR, UNGROUPED_ID, UNGROUPED_NAME,
};
use crate::models::{Validatable, ValidationError};

/// Result type for group service operations
pub type GroupResult<T> = std::result::Result<T, GroupServiceError>;

/// Error type for group service operations
#[derive(Debug, Clone, PartialEq)]
pub enum GroupServiceError {
    /// Group not found
    NotFound(String),
    /// Validation failed
    Validation(ValidationError),
    /// Database error
    Database(String),
    /// Duplicate group name
    DuplicateName(String),
    /// Cannot delete system group
    CannotDeleteSystem(String),
    /// Cannot modify system group
    CannotModifySystem(String),
    /// Server not found for move operation
    ServerNotFound(String),
    /// Import/Export error
    ImportExport(String),
    /// Transaction error
    Transaction(String),
}

impl std::fmt::Display for GroupServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupServiceError::NotFound(id) => write!(f, "Group not found: {}", id),
            GroupServiceError::Validation(e) => write!(f, "Validation error: {}", e),
            GroupServiceError::Database(msg) => write!(f, "Database error: {}", msg),
            GroupServiceError::DuplicateName(name) => {
                write!(f, "Group with name '{}' already exists", name)
            }
            GroupServiceError::CannotDeleteSystem(id) => {
                write!(f, "Cannot delete system group: {}", id)
            }
            GroupServiceError::CannotModifySystem(id) => {
                write!(f, "Cannot modify system group: {}", id)
            }
            GroupServiceError::ServerNotFound(id) => {
                write!(f, "Server not found for move operation: {}", id)
            }
            GroupServiceError::ImportExport(msg) => write!(f, "Import/Export error: {}", msg),
            GroupServiceError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
        }
    }
}

impl std::error::Error for GroupServiceError {}

impl From<ValidationError> for GroupServiceError {
    fn from(e: ValidationError) -> Self {
        GroupServiceError::Validation(e)
    }
}

impl From<LiteError> for GroupServiceError {
    fn from(e: LiteError) -> Self {
        match e {
            LiteError::GroupNotFound(id) => GroupServiceError::NotFound(id),
            LiteError::ServerNotFound(id) => GroupServiceError::ServerNotFound(id),
            _ => GroupServiceError::Database(e.to_string()),
        }
    }
}

impl From<GroupServiceError> for EasySSHErrors {
    fn from(e: GroupServiceError) -> Self {
        match e {
            GroupServiceError::NotFound(id) => {
                EasySSHErrors::Database(CoreDatabaseError::RecordNotFound {
                    table: "groups".to_string(),
                    id,
                })
            }
            GroupServiceError::Database(msg) => {
                EasySSHErrors::Database(CoreDatabaseError::Query(msg))
            }
            GroupServiceError::Validation(msg) => EasySSHErrors::Validation(msg.to_string()),
            GroupServiceError::DuplicateName(name) => {
                EasySSHErrors::Database(CoreDatabaseError::UniqueViolation(name))
            }
            GroupServiceError::ServerNotFound(id) => {
                EasySSHErrors::NotFound(format!("服务器: {}", id))
            }
            GroupServiceError::CannotDeleteSystem(id) => {
                EasySSHErrors::Validation(format!("无法删除系统分组: {}", id))
            }
            GroupServiceError::CannotModifySystem(id) => {
                EasySSHErrors::Validation(format!("无法修改系统分组: {}", id))
            }
            GroupServiceError::ImportExport(msg) => EasySSHErrors::Config(msg),
            GroupServiceError::Transaction(msg) => {
                EasySSHErrors::Database(CoreDatabaseError::Transaction(msg))
            }
        }
    }
}

/// Import result for group operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupImportResult {
    pub total: usize,
    pub imported: usize,
    pub merged: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub success: usize,
    pub failed: usize,
    pub errors: Vec<(String, String)>, // (id, error message)
}

/// Group service for managing server groups
pub struct GroupService {
    db: Arc<Mutex<Database>>,
}

impl GroupService {
    /// Create a new group service instance
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Initialize default groups if they don't exist
    pub fn initialize_default_groups(&self) -> GroupResult<()> {
        let existing = self.list_groups()?;

        // Only create defaults if no groups exist
        if existing.is_empty() {
            let defaults = vec![
                ("开发", "#4A90D9"),
                ("测试", "#F5A623"),
                ("生产", "#D0021B"),
            ];

            for (name, color) in defaults {
                let group = Group::new(name.to_string(), color.to_string());
                self.create_group_internal(&group)?;
            }
        }

        Ok(())
    }

    /// Create a new group
    pub fn create_group(&self, request: CreateGroupRequest) -> GroupResult<Group> {
        let group = Group::new(request.name, request.color);
        group.validate()?;

        // Check for duplicate name
        if self.is_duplicate_name(&group.name, None)? {
            return Err(GroupServiceError::DuplicateName(group.name));
        }

        self.create_group_internal(&group)?;
        Ok(group)
    }

    /// Internal method to create a group without validation
    fn create_group_internal(&self, group: &Group) -> GroupResult<()> {
        let new_group = NewGroup {
            id: group.id.clone(),
            name: group.name.clone(),
            color: group.color.clone(),
        };

        self.db
            .lock()
            .unwrap()
            .add_group(&new_group)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Ok(())
    }

    /// Update an existing group
    pub fn update_group(&self, id: &str, request: UpdateGroupRequest) -> GroupResult<Group> {
        // Check if it's a system group
        if self.is_system_group(id) {
            return Err(GroupServiceError::CannotModifySystem(id.to_string()));
        }

        let existing = self.get_group(id)?;

        // Check for duplicate name if name is being changed
        if let Some(ref name) = request.name {
            if name != &existing.name && self.is_duplicate_name(name, Some(id))? {
                return Err(GroupServiceError::DuplicateName(name.clone()));
            }
        }

        let updated = Group {
            id: existing.id,
            name: request.name.unwrap_or(existing.name),
            color: request.color.unwrap_or(existing.color),
            created_at: existing.created_at,
            updated_at: Utc::now(),
            schema_version: existing.schema_version,
        };

        updated.validate()?;

        let update_record = UpdateGroup {
            id: updated.id.clone(),
            name: Some(updated.name.clone()),
            color: Some(updated.color.clone()),
        };

        self.db
            .lock()
            .unwrap()
            .update_group(&update_record)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Ok(updated)
    }

    /// Delete a group by ID
    /// Servers in this group will be moved to ungrouped
    pub fn delete_group(&self, id: &str) -> GroupResult<()> {
        // Check if it's a system group
        if self.is_system_group(id) {
            return Err(GroupServiceError::CannotDeleteSystem(id.to_string()));
        }

        // Check if group exists
        self.get_group(id)?;

        // Move servers to ungrouped before deleting
        self.move_servers_to_ungrouped(id)?;

        // Delete the group
        self.db
            .lock()
            .unwrap()
            .delete_group(id)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Ok(())
    }

    /// Force delete a group and all its servers (use with caution)
    pub fn delete_group_with_servers(&self, id: &str) -> GroupResult<usize> {
        if self.is_system_group(id) {
            return Err(GroupServiceError::CannotDeleteSystem(id.to_string()));
        }

        self.get_group(id)?;

        // Get all servers in this group
        let servers = self.get_servers_in_group(id)?;
        let count = servers.len();

        // Delete all servers first
        for server in servers {
            self.db
                .lock()
                .unwrap()
                .delete_server(&server.id)
                .map_err(|e| GroupServiceError::Database(e.to_string()))?;
        }

        // Delete the group
        self.db
            .lock()
            .unwrap()
            .delete_group(id)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Ok(count)
    }

    /// Get a single group by ID
    pub fn get_group(&self, id: &str) -> GroupResult<Group> {
        if id == UNGROUPED_ID {
            return Ok(self.get_ungrouped_system_group());
        }

        let record = self
            .db
            .lock()
            .unwrap()
            .get_group(id)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Self::record_to_group(record)
    }

    /// Get the ungrouped system group
    fn get_ungrouped_system_group(&self) -> Group {
        Group::with_id(
            UNGROUPED_ID.to_string(),
            UNGROUPED_NAME.to_string(),
            UNGROUPED_COLOR.to_string(),
        )
    }

    /// Check if a group is a system group
    fn is_system_group(&self, id: &str) -> bool {
        id == UNGROUPED_ID
    }

    /// List all groups including the ungrouped system group
    pub fn list_groups(&self) -> GroupResult<Vec<Group>> {
        let records = self
            .db
            .lock()
            .unwrap()
            .get_groups()
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        let mut groups: Vec<Group> = records
            .into_iter()
            .map(Self::record_to_group)
            .collect::<GroupResult<Vec<_>>>()?;

        // Add ungrouped system group at the beginning
        groups.insert(0, self.get_ungrouped_system_group());

        Ok(groups)
    }

    /// Get groups with server counts
    pub fn get_group_stats(&self) -> GroupResult<Vec<GroupStats>> {
        let groups = self.list_groups()?;
        let mut stats = Vec::new();

        for group in groups {
            let count = if group.is_ungrouped() {
                // Count servers without a group
                self.count_ungrouped_servers()?
            } else {
                self.count_servers_in_group(&group.id)?
            };

            stats.push(GroupStats {
                group,
                server_count: count,
            });
        }

        Ok(stats)
    }

    /// Get a group with its servers
    pub fn get_group_with_servers(&self, id: &str) -> GroupResult<GroupWithServers> {
        let group = self.get_group(id)?;
        let servers = self.get_servers_in_group(id)?;

        Ok(GroupWithServers { group, servers })
    }

    /// Get all groups with their servers
    pub fn get_all_groups_with_servers(&self) -> GroupResult<Vec<GroupWithServers>> {
        let groups = self.list_groups()?;
        let mut result = Vec::new();

        for group in groups {
            let servers = self.get_servers_in_group(&group.id)?;
            result.push(GroupWithServers {
                group: group.clone(),
                servers,
            });
        }

        Ok(result)
    }

    /// Move a server to a different group
    pub fn move_server_to_group(&self, request: MoveServerRequest) -> GroupResult<()> {
        let server = self
            .db
            .lock()
            .unwrap()
            .get_server(&request.server_id)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        // If target group is specified, verify it exists
        if let Some(ref group_id) = request.target_group_id {
            if group_id != UNGROUPED_ID {
                self.get_group(group_id)?;
            }
        }

        // Update server's group_id
        let update = crate::db::UpdateServer {
            id: server.id.clone(),
            name: None,
            host: None,
            port: None,
            username: None,
            auth_type: None,
            identity_file: None,
            group_id: request.target_group_id,
            status: None,
        };

        self.db
            .lock()
            .unwrap()
            .update_server(&update)
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        Ok(())
    }

    /// Batch move servers to a group
    pub fn batch_move_servers(
        &self,
        server_ids: &[String],
        target_group_id: Option<GroupId>,
    ) -> GroupResult<BatchOperationResult> {
        let mut result = BatchOperationResult {
            success: 0,
            failed: 0,
            errors: Vec::new(),
        };

        // Verify target group exists if specified
        if let Some(ref group_id) = target_group_id {
            if group_id != UNGROUPED_ID {
                self.get_group(group_id)?;
            }
        }

        for server_id in server_ids {
            let request = MoveServerRequest {
                server_id: server_id.clone(),
                target_group_id: target_group_id.clone(),
            };

            match self.move_server_to_group(request) {
                Ok(_) => result.success += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((server_id.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Move all servers from one group to another
    pub fn merge_groups(&self, source_group_id: &str, target_group_id: &str) -> GroupResult<usize> {
        if source_group_id == target_group_id {
            return Ok(0);
        }

        // Verify both groups exist
        self.get_group(source_group_id)?;
        self.get_group(target_group_id)?;

        let servers = self.get_servers_in_group(source_group_id)?;
        let count = servers.len();

        // Move all servers
        for server in &servers {
            let request = MoveServerRequest {
                server_id: server.id.clone(),
                target_group_id: Some(target_group_id.to_string()),
            };
            self.move_server_to_group(request)?;
        }

        Ok(count)
    }

    /// Search groups by name
    pub fn search_groups(&self, keyword: &str) -> GroupResult<Vec<Group>> {
        let keyword_lower = keyword.to_lowercase();
        let all = self.list_groups()?;

        Ok(all
            .into_iter()
            .filter(|g| g.name.to_lowercase().contains(&keyword_lower))
            .collect())
    }

    /// Export groups to JSON
    pub fn export_to_json(&self, include_servers: bool) -> GroupResult<String> {
        let data = if include_servers {
            let groups_with_servers = self.get_all_groups_with_servers()?;
            serde_json::to_string_pretty(&groups_with_servers)
        } else {
            let groups = self.list_groups()?;
            serde_json::to_string_pretty(&groups)
        };

        data.map_err(|e| GroupServiceError::ImportExport(e.to_string()))
    }

    /// Import groups from JSON
    pub fn import_from_json(
        &self,
        json: &str,
        merge_existing: bool,
    ) -> GroupResult<GroupImportResult> {
        let groups: Vec<Group> = serde_json::from_str(json)
            .map_err(|e| GroupServiceError::ImportExport(e.to_string()))?;

        let mut result = GroupImportResult {
            total: groups.len(),
            imported: 0,
            merged: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        for group in groups {
            // Skip system groups
            if group.is_ungrouped() {
                result.skipped += 1;
                continue;
            }

            let existing = self
                .db
                .lock()
                .unwrap()
                .get_groups()
                .map_err(|e| GroupServiceError::Database(e.to_string()))?
                .into_iter()
                .find(|g| g.name == group.name);

            if let Some(existing_group) = existing {
                if merge_existing {
                    // Merge: update color if provided
                    let update = UpdateGroupRequest {
                        name: None,
                        color: Some(group.color),
                    };
                    if let Err(e) = self.update_group(&existing_group.id, update) {
                        result
                            .errors
                            .push(format!("Failed to merge {}: {}", group.name, e));
                    } else {
                        result.merged += 1;
                    }
                } else {
                    result.skipped += 1;
                }
            } else {
                // Create new group
                let request = CreateGroupRequest {
                    name: group.name,
                    color: group.color,
                };
                if let Err(e) = self.create_group(request) {
                    result.errors.push(format!("Failed to import group: {}", e));
                } else {
                    result.imported += 1;
                }
            }
        }

        Ok(result)
    }

    /// Batch update group colors
    pub fn batch_update_colors(
        &self,
        updates: HashMap<String, String>,
    ) -> GroupResult<BatchOperationResult> {
        let mut result = BatchOperationResult {
            success: 0,
            failed: 0,
            errors: Vec::new(),
        };

        for (id, color) in updates {
            if self.is_system_group(&id) {
                result.failed += 1;
                result
                    .errors
                    .push((id.clone(), "Cannot modify system group".to_string()));
                continue;
            }

            let update = UpdateGroupRequest {
                name: None,
                color: Some(color),
            };

            match self.update_group(&id, update) {
                Ok(_) => result.success += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((id, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    /// Rename a group
    pub fn rename_group(&self, id: &str, new_name: String) -> GroupResult<Group> {
        let update = UpdateGroupRequest {
            name: Some(new_name),
            color: None,
        };
        self.update_group(id, update)
    }

    /// Change group color
    pub fn change_group_color(&self, id: &str, new_color: String) -> GroupResult<Group> {
        let update = UpdateGroupRequest {
            name: None,
            color: Some(new_color),
        };
        self.update_group(id, update)
    }

    /// Get group count
    pub fn count_groups(&self) -> GroupResult<usize> {
        // Subtract 1 to exclude ungrouped system group
        let count = self.list_groups()?.len();
        Ok(count.saturating_sub(1))
    }

    /// Check if group name already exists
    fn is_duplicate_name(&self, name: &str, exclude_id: Option<&str>) -> GroupResult<bool> {
        let groups = self.list_groups()?;
        Ok(groups
            .iter()
            .any(|g| g.name == name && exclude_id.map(|id| g.id != id).unwrap_or(true)))
    }

    /// Get servers in a group
    fn get_servers_in_group(&self, group_id: &str) -> GroupResult<Vec<ServerReference>> {
        let all_servers = self
            .db
            .lock()
            .unwrap()
            .get_servers()
            .map_err(|e| GroupServiceError::Database(e.to_string()))?;

        let filtered: Vec<ServerReference> = all_servers
            .into_iter()
            .filter(|s| {
                if group_id == UNGROUPED_ID {
                    s.group_id.is_none()
                } else {
                    s.group_id.as_ref() == Some(&group_id.to_string())
                }
            })
            .map(|s| ServerReference {
                id: s.id,
                name: s.name,
                host: s.host,
                port: s.port,
            })
            .collect();

        Ok(filtered)
    }

    /// Count servers in a group
    fn count_servers_in_group(&self, group_id: &str) -> GroupResult<usize> {
        let servers = self.get_servers_in_group(group_id)?;
        Ok(servers.len())
    }

    /// Count ungrouped servers
    fn count_ungrouped_servers(&self) -> GroupResult<usize> {
        self.count_servers_in_group(UNGROUPED_ID)
    }

    /// Move all servers from a group to ungrouped
    fn move_servers_to_ungrouped(&self, group_id: &str) -> GroupResult<()> {
        let servers = self.get_servers_in_group(group_id)?;

        for server in servers {
            let request = MoveServerRequest {
                server_id: server.id,
                target_group_id: None,
            };
            self.move_server_to_group(request)?;
        }

        Ok(())
    }

    /// Convert a database record to Group model
    fn record_to_group(record: GroupRecord) -> GroupResult<Group> {
        let created_at = record
            .created_at
            .parse::<i64>()
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default())
            .unwrap_or_else(|_| Utc::now());

        let updated_at = record
            .updated_at
            .parse::<i64>()
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default())
            .unwrap_or_else(|_| Utc::now());

        Ok(Group {
            id: record.id,
            name: record.name,
            color: record.color,
            created_at,
            updated_at,
            schema_version: 1,
        })
    }

    /// Execute operations within a transaction
    pub fn with_transaction<F, T>(&self, operations: F) -> GroupResult<T>
    where
        F: FnOnce(&GroupService) -> GroupResult<T>,
    {
        // Note: This is a simplified transaction implementation
        // In production, you'd use proper database transactions
        operations(self)
    }
}

/// Async wrapper for GroupService
pub struct AsyncGroupService {
    inner: Arc<GroupService>,
}

impl AsyncGroupService {
    /// Create a new async group service
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            inner: Arc::new(GroupService::new(db)),
        }
    }

    /// Initialize default groups
    pub fn initialize_default_groups(&self) -> GroupResult<()> {
        self.inner.initialize_default_groups()
    }

    /// Create a group
    pub fn create_group(&self, request: CreateGroupRequest) -> GroupResult<Group> {
        self.inner.create_group(request)
    }

    /// Update a group
    pub fn update_group(&self, id: &str, request: UpdateGroupRequest) -> GroupResult<Group> {
        self.inner.update_group(id, request)
    }

    /// Delete a group
    pub fn delete_group(&self, id: &str) -> GroupResult<()> {
        self.inner.delete_group(id)
    }

    /// Get a group
    pub fn get_group(&self, id: &str) -> GroupResult<Group> {
        self.inner.get_group(id)
    }

    /// List all groups
    pub fn list_groups(&self) -> GroupResult<Vec<Group>> {
        self.inner.list_groups()
    }

    /// Get group stats
    pub fn get_group_stats(&self) -> GroupResult<Vec<GroupStats>> {
        self.inner.get_group_stats()
    }

    /// Move server to group
    pub fn move_server_to_group(&self, request: MoveServerRequest) -> GroupResult<()> {
        self.inner.move_server_to_group(request)
    }

    /// Search groups
    pub fn search_groups(&self, keyword: &str) -> GroupResult<Vec<Group>> {
        self.inner.search_groups(keyword)
    }

    /// Export to JSON
    pub fn export_to_json(&self, include_servers: bool) -> GroupResult<String> {
        self.inner.export_to_json(include_servers)
    }

    /// Import from JSON
    pub fn import_from_json(
        &self,
        json: &str,
        merge_existing: bool,
    ) -> GroupResult<GroupImportResult> {
        self.inner.import_from_json(json, merge_existing)
    }

    /// Merge groups
    pub fn merge_groups(&self, source_group_id: &str, target_group_id: &str) -> GroupResult<usize> {
        self.inner.merge_groups(source_group_id, target_group_id)
    }

    /// Batch move servers
    pub fn batch_move_servers(
        &self,
        server_ids: &[String],
        target_group_id: Option<GroupId>,
    ) -> GroupResult<BatchOperationResult> {
        self.inner.batch_move_servers(server_ids, target_group_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> Arc<Mutex<Database>> {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).unwrap();
        db.init().unwrap();
        Arc::new(Mutex::new(db))
    }

    #[test]
    fn test_create_group() {
        let service = GroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Test Group".to_string(),
            color: "#4A90D9".to_string(),
        };

        let group = service.create_group(request).unwrap();
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.color, "#4A90D9");
    }

    #[test]
    fn test_create_group_duplicate_name() {
        let service = GroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Test Group".to_string(),
            color: "#4A90D9".to_string(),
        };

        service.create_group(request.clone()).unwrap();
        let result = service.create_group(request);

        assert!(matches!(result, Err(GroupServiceError::DuplicateName(_))));
    }

    #[test]
    fn test_get_group() {
        let service = GroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Test Group".to_string(),
            color: "#4A90D9".to_string(),
        };

        let created = service.create_group(request).unwrap();
        let retrieved = service.get_group(&created.id).unwrap();

        assert_eq!(retrieved.name, created.name);
        assert_eq!(retrieved.id, created.id);
    }

    #[test]
    fn test_get_ungrouped_system_group() {
        let service = GroupService::new(create_test_db());

        let ungrouped = service.get_group(UNGROUPED_ID).unwrap();
        assert!(ungrouped.is_ungrouped());
        assert_eq!(ungrouped.name, UNGROUPED_NAME);
    }

    #[test]
    fn test_update_group() {
        let service = GroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Test Group".to_string(),
            color: "#4A90D9".to_string(),
        };

        let created = service.create_group(request).unwrap();

        let update = UpdateGroupRequest {
            name: Some("Updated Group".to_string()),
            color: None,
        };

        let updated = service.update_group(&created.id, update).unwrap();
        assert_eq!(updated.name, "Updated Group");
        assert_eq!(updated.color, "#4A90D9"); // Unchanged
    }

    #[test]
    fn test_cannot_modify_ungrouped() {
        let service = GroupService::new(create_test_db());

        let update = UpdateGroupRequest {
            name: Some("New Name".to_string()),
            color: None,
        };

        let result = service.update_group(UNGROUPED_ID, update);
        assert!(matches!(
            result,
            Err(GroupServiceError::CannotModifySystem(_))
        ));
    }

    #[test]
    fn test_delete_group() {
        let service = GroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Test Group".to_string(),
            color: "#4A90D9".to_string(),
        };

        let created = service.create_group(request).unwrap();
        service.delete_group(&created.id).unwrap();

        let result = service.get_group(&created.id);
        assert!(matches!(result, Err(GroupServiceError::NotFound(_))));
    }

    #[test]
    fn test_cannot_delete_ungrouped() {
        let service = GroupService::new(create_test_db());

        let result = service.delete_group(UNGROUPED_ID);
        assert!(matches!(
            result,
            Err(GroupServiceError::CannotDeleteSystem(_))
        ));
    }

    #[test]
    fn test_list_groups_includes_ungrouped() {
        let service = GroupService::new(create_test_db());

        let groups = service.list_groups().unwrap();
        assert!(groups.iter().any(|g| g.is_ungrouped()));
    }

    #[test]
    fn test_search_groups() {
        let service = GroupService::new(create_test_db());

        service
            .create_group(CreateGroupRequest {
                name: "Production Servers".to_string(),
                color: "#D0021B".to_string(),
            })
            .unwrap();

        service
            .create_group(CreateGroupRequest {
                name: "Development".to_string(),
                color: "#4A90D9".to_string(),
            })
            .unwrap();

        let results = service.search_groups("prod").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Production Servers");
    }

    #[test]
    fn test_initialize_default_groups() {
        let service = GroupService::new(create_test_db());

        service.initialize_default_groups().unwrap();

        let groups = service.list_groups().unwrap();
        assert!(groups.iter().any(|g| g.name == "开发"));
        assert!(groups.iter().any(|g| g.name == "测试"));
        assert!(groups.iter().any(|g| g.name == "生产"));
    }

    #[test]
    fn test_initialize_only_when_empty() {
        let service = GroupService::new(create_test_db());

        // First initialization
        service.initialize_default_groups().unwrap();
        let count_after_first = service.list_groups().unwrap().len();

        // Second initialization should not add more groups
        service.initialize_default_groups().unwrap();
        let count_after_second = service.list_groups().unwrap().len();

        assert_eq!(count_after_first, count_after_second);
    }

    #[test]
    fn test_count_groups() {
        let service = GroupService::new(create_test_db());

        // Should be 0 (excluding ungrouped)
        assert_eq!(service.count_groups().unwrap(), 0);

        service
            .create_group(CreateGroupRequest {
                name: "Group 1".to_string(),
                color: "#4A90D9".to_string(),
            })
            .unwrap();

        assert_eq!(service.count_groups().unwrap(), 1);
    }

    #[test]
    fn test_rename_group() {
        let service = GroupService::new(create_test_db());

        let created = service
            .create_group(CreateGroupRequest {
                name: "Old Name".to_string(),
                color: "#4A90D9".to_string(),
            })
            .unwrap();

        let renamed = service
            .rename_group(&created.id, "New Name".to_string())
            .unwrap();
        assert_eq!(renamed.name, "New Name");
    }

    #[test]
    fn test_change_group_color() {
        let service = GroupService::new(create_test_db());

        let created = service
            .create_group(CreateGroupRequest {
                name: "Test".to_string(),
                color: "#4A90D9".to_string(),
            })
            .unwrap();

        let updated = service
            .change_group_color(&created.id, "#D0021B".to_string())
            .unwrap();
        assert_eq!(updated.color, "#D0021B");
    }

    #[test]
    fn test_group_export_import() {
        let service = GroupService::new(create_test_db());

        service
            .create_group(CreateGroupRequest {
                name: "Production".to_string(),
                color: "#D0021B".to_string(),
            })
            .unwrap();

        let json = service.export_to_json(false).unwrap();
        assert!(json.contains("Production"));

        // Import should skip existing
        let result = service.import_from_json(&json, false).unwrap();
        assert_eq!(result.total, 4); // 3 created + 1 ungrouped
        assert_eq!(result.skipped, 3); // defaults + production
    }

    #[test]
    fn test_batch_update_colors() {
        let service = GroupService::new(create_test_db());

        let g1 = service
            .create_group(CreateGroupRequest {
                name: "Group 1".to_string(),
                color: "#4A90D9".to_string(),
            })
            .unwrap();

        let g2 = service
            .create_group(CreateGroupRequest {
                name: "Group 2".to_string(),
                color: "#F5A623".to_string(),
            })
            .unwrap();

        let mut updates = HashMap::new();
        updates.insert(g1.id.clone(), "#111111".to_string());
        updates.insert(g2.id.clone(), "#222222".to_string());

        let result = service.batch_update_colors(updates).unwrap();
        assert_eq!(result.success, 2);

        let updated_g1 = service.get_group(&g1.id).unwrap();
        assert_eq!(updated_g1.color, "#111111");
    }

    #[test]
    fn test_error_conversions() {
        let err = GroupServiceError::NotFound("test".to_string());
        let easy_err: EasySSHErrors = err.into();
        assert!(matches!(easy_err, EasySSHErrors::Database(_)));

        let err = GroupServiceError::DuplicateName("test".to_string());
        let easy_err: EasySSHErrors = err.into();
        assert!(matches!(easy_err, EasySSHErrors::Database(_)));
    }

    #[test]
    fn test_batch_operation_result() {
        let result = BatchOperationResult {
            success: 5,
            failed: 2,
            errors: vec![("id1".to_string(), "error1".to_string())],
        };

        assert_eq!(result.success, 5);
        assert_eq!(result.failed, 2);
    }

    #[test]
    fn test_group_import_result() {
        let result = GroupImportResult {
            total: 10,
            imported: 6,
            merged: 2,
            skipped: 2,
            errors: vec![],
        };

        assert_eq!(result.total, 10);
        assert_eq!(result.imported, 6);
        assert_eq!(result.merged, 2);
        assert_eq!(result.skipped, 2);
    }

    #[test]
    fn test_async_group_service_wrapper() {
        let service = AsyncGroupService::new(create_test_db());

        let request = CreateGroupRequest {
            name: "Async Test".to_string(),
            color: "#4A90D9".to_string(),
        };

        let group = service.create_group(request).unwrap();
        assert_eq!(group.name, "Async Test");

        let groups = service.list_groups().unwrap();
        assert!(groups.iter().any(|g| g.name == "Async Test"));
    }
}
