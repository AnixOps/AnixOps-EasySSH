//! Conflict resolution for sync

use crate::error::LiteError;
use serde::{Deserialize, Serialize};

use super::types::{SyncDocument, SyncDocumentType};

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncConflict {
    pub document_id: String,
    pub doc_type: SyncDocumentType,
    pub local_version: SyncDocument,
    pub remote_version: SyncDocument,
    pub resolution: Option<SyncConflictResolution>,
    pub detected_at: i64,
    pub field_conflicts: Vec<FieldConflict>,
}

/// Field-level conflict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldConflict {
    pub field_name: String,
    pub local_value: serde_json::Value,
    pub remote_value: serde_json::Value,
    pub resolution: Option<serde_json::Value>,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictResolution {
    UseLocal,
    UseRemote,
    Merge,
    KeepBoth,
    Interactive,
    TimestampWins,
    DevicePriority { device_order: Vec<String> },
    Skip,
}

impl Default for SyncConflictResolution {
    fn default() -> Self {
        SyncConflictResolution::Merge
    }
}

/// Conflict resolver
pub struct ConflictResolver;

impl ConflictResolver {
    /// Detect field-level conflicts between local and remote data
    pub fn detect_field_conflicts(
        local_data: &[u8],
        remote_data: &[u8],
        doc_type: &SyncDocumentType,
    ) -> Result<Vec<FieldConflict>, LiteError> {
        let mut conflicts = Vec::new();

        if !doc_type.supports_field_merge() {
            return Ok(conflicts);
        }

        let local: serde_json::Value = serde_json::from_slice(local_data)?;
        let remote: serde_json::Value = serde_json::from_slice(remote_data)?;

        if let (Some(local_obj), Some(remote_obj)) = (local.as_object(), remote.as_object()) {
            for (key, local_value) in local_obj {
                if let Some(remote_value) = remote_obj.get(key) {
                    if local_value != remote_value {
                        conflicts.push(FieldConflict {
                            field_name: key.clone(),
                            local_value: local_value.clone(),
                            remote_value: remote_value.clone(),
                            resolution: None,
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Try to merge field conflicts automatically
    pub fn try_merge_fields(
        local: &serde_json::Value,
        remote: &serde_json::Value,
        field_conflicts: &[FieldConflict],
        doc_type: &SyncDocumentType,
    ) -> Result<Option<serde_json::Value>, LiteError> {
        let mut merged = local.clone();
        let mut merge_count = 0;

        for field_conflict in field_conflicts {
            if let Some(merged_value) = Self::merge_field_value(
                &field_conflict.field_name,
                &field_conflict.local_value,
                &field_conflict.remote_value,
                doc_type,
            ) {
                if let Some(obj) = merged.as_object_mut() {
                    obj.insert(field_conflict.field_name.clone(), merged_value);
                    merge_count += 1;
                }
            }
        }

        if merge_count == field_conflicts.len() {
            Ok(Some(merged))
        } else {
            Ok(None)
        }
    }

    /// Merge a single field value based on document type
    fn merge_field_value(
        field_name: &str,
        local: &serde_json::Value,
        remote: &serde_json::Value,
        doc_type: &SyncDocumentType,
    ) -> Option<serde_json::Value> {
        match doc_type {
            SyncDocumentType::Host => match field_name {
                "tags" | "aliases" => {
                    if let (Some(local_arr), Some(remote_arr)) =
                        (local.as_array(), remote.as_array())
                    {
                        let mut merged = local_arr.clone();
                        for item in remote_arr {
                            if !merged.contains(item) {
                                merged.push(item.clone());
                            }
                        }
                        return Some(serde_json::Value::Array(merged));
                    }
                    None
                }
                "notes" | "description" => {
                    if let (Some(local_str), Some(remote_str)) = (local.as_str(), remote.as_str()) {
                        let merged = format!("{}\n---\n{}", local_str, remote_str);
                        return Some(serde_json::Value::String(merged));
                    }
                    None
                }
                _ => None,
            },
            SyncDocumentType::Group | SyncDocumentType::Tag => match field_name {
                "description" | "notes" => {
                    if local.is_null() || local.as_str() == Some("") {
                        Some(remote.clone())
                    } else if remote.is_null() || remote.as_str() == Some("") {
                        Some(local.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            SyncDocumentType::Snippet => {
                if field_name == "tags" {
                    if let (Some(local_arr), Some(remote_arr)) =
                        (local.as_array(), remote.as_array())
                    {
                        let mut merged = local_arr.clone();
                        for item in remote_arr {
                            if !merged.contains(item) {
                                merged.push(item.clone());
                            }
                        }
                        return Some(serde_json::Value::Array(merged));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Resolve conflict using specified strategy
    pub fn resolve_with_strategy(
        conflict: &SyncConflict,
        strategy: &SyncConflictResolution,
    ) -> SyncConflictResolution {
        match strategy {
            SyncConflictResolution::UseLocal => SyncConflictResolution::UseLocal,
            SyncConflictResolution::UseRemote => SyncConflictResolution::UseRemote,
            SyncConflictResolution::TimestampWins => {
                if conflict.local_version.timestamp > conflict.remote_version.timestamp {
                    SyncConflictResolution::UseLocal
                } else {
                    SyncConflictResolution::UseRemote
                }
            }
            SyncConflictResolution::DevicePriority { device_order } => {
                let local_priority = device_order
                    .iter()
                    .position(|d| d == &conflict.local_version.device_id)
                    .unwrap_or(usize::MAX);
                let remote_priority = device_order
                    .iter()
                    .position(|d| d == &conflict.remote_version.device_id)
                    .unwrap_or(usize::MAX);

                if local_priority <= remote_priority {
                    SyncConflictResolution::UseLocal
                } else {
                    SyncConflictResolution::UseRemote
                }
            }
            _ => strategy.clone(),
        }
    }
}
