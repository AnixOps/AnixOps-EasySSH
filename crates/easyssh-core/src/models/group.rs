//! Group models for EasySSH
//!
//! This module provides data structures for server group management.
//! Groups are used to organize servers in a flat (non-nested) structure for the Lite edition.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Validatable, ValidationError, MAX_NAME_LENGTH};

/// Unique identifier for groups
pub type GroupId = String;

/// ID for the default "ungrouped" system group
pub const UNGROUPED_ID: &str = "_ungrouped";

/// Name for the default "ungrouped" system group
pub const UNGROUPED_NAME: &str = "未分组";

/// Color for the default "ungrouped" system group (gray)
pub const UNGROUPED_COLOR: &str = "#9CA3AF";

/// Validates a hex color string
///
/// # Arguments
/// * `color` - The color string to validate
///
/// # Returns
/// * `true` if valid (e.g., "#4A90D9", "#4a90d9", "#FFF")
/// * `false` otherwise
pub fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }

    let hex_part = &color[1..];
    let len = hex_part.len();

    // Support both 3-digit and 6-digit hex colors
    if len != 3 && len != 6 {
        return false;
    }

    hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

/// Represents a server group
///
/// Groups are used to organize servers in a flat structure.
/// Each group has a name, color, and creation timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    /// Unique identifier for the group
    pub id: GroupId,
    /// Display name of the group
    pub name: String,
    /// Hex color code for UI display (e.g., "#4A90D9")
    pub color: String,
    /// When the group was created
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version for migrations
    #[serde(default)]
    pub schema_version: u32,
}

impl Group {
    /// Creates a new group with the specified name and color
    ///
    /// # Arguments
    /// * `name` - The display name for the group
    /// * `color` - Hex color code (e.g., "#4A90D9")
    ///
    /// # Returns
    /// A new Group instance with a generated ID and current timestamp
    ///
    /// # Example
    /// ```
    /// use easyssh_core::models::group::Group;
    ///
    /// let group = Group::new("Production".to_string(), "#FF5733".to_string());
    /// assert_eq!(group.name, "Production");
    /// ```
    pub fn new(name: String, color: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            color,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Creates a group with a specific ID (used for system groups and imports)
    ///
    /// # Arguments
    /// * `id` - The unique identifier for the group
    /// * `name` - The display name for the group
    /// * `color` - Hex color code
    pub fn with_id(id: GroupId, name: String, color: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            color,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Checks if this is the default "ungrouped" system group
    pub fn is_ungrouped(&self) -> bool {
        self.id == UNGROUPED_ID
    }

    /// Update the group and refresh the updated_at timestamp
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }
}

impl Validatable for Group {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate name
        if self.name.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "name".to_string(),
                message: "Group name cannot be empty".to_string(),
            });
        }
        if self.name.len() > MAX_NAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "name".to_string(),
                message: format!("Group name too long (max {} characters)", MAX_NAME_LENGTH),
            });
        }

        // Validate color
        if !is_valid_hex_color(&self.color) {
            return Err(ValidationError::InvalidFormat {
                field: "color".to_string(),
                expected: "valid hex color (e.g., #4A90D9 or #FFF)".to_string(),
            });
        }

        Ok(())
    }
}

/// Preset group definitions for new installations
pub const PRESET_GROUPS: &[(&str, &str)] = &[
    ("开发", "#4A90D9"), // Blue
    ("测试", "#F5A623"), // Orange
    ("生产", "#D0021B"), // Red
];

/// Default color palette for group selection
pub const DEFAULT_COLOR_PALETTE: &[&str] = &[
    "#EF4444", // Red
    "#F97316", // Orange
    "#F59E0B", // Amber
    "#84CC16", // Lime
    "#22C55E", // Green
    "#10B981", // Emerald
    "#14B8A6", // Teal
    "#06B6D4", // Cyan
    "#0EA5E9", // Sky
    "#3B82F6", // Blue
    "#6366F1", // Indigo
    "#8B5CF6", // Violet
    "#A855F7", // Purple
    "#D946EF", // Fuchsia
    "#EC4899", // Pink
    "#F43F5E", // Rose
    "#6B7280", // Gray
    "#9CA3AF", // Light Gray
];

/// Group with server count information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupStats {
    /// The group information
    pub group: Group,
    /// Number of servers in this group
    pub server_count: usize,
}

/// Represents a server reference (minimal info for group display)
///
/// This is a lightweight struct to avoid circular dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerReference {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
}

/// Group with its associated servers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupWithServers {
    /// The group information
    pub group: Group,
    /// List of servers in this group
    pub servers: Vec<ServerReference>,
}

/// Request to create a new group
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub color: String,
}

/// Request to update an existing group
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

/// Request to move a server to a different group
#[derive(Debug, Clone, Deserialize)]
pub struct MoveServerRequest {
    pub server_id: String,
    pub target_group_id: Option<GroupId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_new() {
        let group = Group::new("Test Group".to_string(), "#4A90D9".to_string());
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.color, "#4A90D9");
        assert!(!group.id.is_empty());
    }

    #[test]
    fn test_group_with_id() {
        let group = Group::with_id(
            "custom-id".to_string(),
            "Custom Group".to_string(),
            "#FF5733".to_string(),
        );
        assert_eq!(group.id, "custom-id");
        assert_eq!(group.name, "Custom Group");
    }

    #[test]
    fn test_is_ungrouped() {
        let ungrouped = Group::with_id(
            UNGROUPED_ID.to_string(),
            UNGROUPED_NAME.to_string(),
            UNGROUPED_COLOR.to_string(),
        );
        assert!(ungrouped.is_ungrouped());

        let regular = Group::new("Regular".to_string(), "#4A90D9".to_string());
        assert!(!regular.is_ungrouped());
    }

    #[test]
    fn test_is_valid_hex_color() {
        assert!(is_valid_hex_color("#4A90D9"));
        assert!(is_valid_hex_color("#4a90d9"));
        assert!(is_valid_hex_color("#FFF"));
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#123ABC"));

        assert!(!is_valid_hex_color("4A90D9")); // Missing #
        assert!(!is_valid_hex_color("#GGGGGG")); // Invalid hex chars
        assert!(!is_valid_hex_color("#4A90D")); // Wrong length
        assert!(!is_valid_hex_color("#4A90D99")); // Wrong length
        assert!(!is_valid_hex_color("")); // Empty
    }

    #[test]
    fn test_group_validation_valid() {
        let valid_group = Group::new("Test".to_string(), "#4A90D9".to_string());
        assert!(valid_group.validate().is_ok());
    }

    #[test]
    fn test_group_validation_empty_name() {
        let invalid_group = Group::new("".to_string(), "#4A90D9".to_string());
        assert!(matches!(
            invalid_group.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_group_validation_long_name() {
        let invalid_group = Group::new("a".repeat(101), "#4A90D9".to_string());
        assert!(matches!(
            invalid_group.validate(),
            Err(ValidationError::InvalidField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_group_validation_invalid_color() {
        let invalid_group = Group::new("Test".to_string(), "invalid".to_string());
        assert!(matches!(
            invalid_group.validate(),
            Err(ValidationError::InvalidFormat { field, .. }) if field == "color"
        ));
    }

    #[test]
    fn test_group_update() {
        let mut group = Group::new("Original".to_string(), "#4A90D9".to_string());
        let old_updated = group.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        group.update(|g| {
            g.name = "Updated".to_string();
        });

        assert_eq!(group.name, "Updated");
        assert!(group.updated_at > old_updated);
    }

    #[test]
    fn test_group_schema_version() {
        let group = Group::new("Test".to_string(), "#4A90D9".to_string());
        assert_eq!(group.schema_version, 1);
    }

    #[test]
    fn test_preset_groups() {
        assert_eq!(PRESET_GROUPS.len(), 3);
        assert_eq!(PRESET_GROUPS[0].0, "开发");
        assert_eq!(PRESET_GROUPS[1].0, "测试");
        assert_eq!(PRESET_GROUPS[2].0, "生产");
    }

    #[test]
    fn test_default_color_palette() {
        assert_eq!(DEFAULT_COLOR_PALETTE.len(), 18);
        // All colors should be valid hex colors
        for color in DEFAULT_COLOR_PALETTE {
            assert!(is_valid_hex_color(color), "Invalid color: {}", color);
        }
    }

    #[test]
    fn test_group_serialization() {
        let group = Group::with_id(
            "test-id".to_string(),
            "Test Group".to_string(),
            "#4A90D9".to_string(),
        );

        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Test Group"));
        assert!(json.contains("#4A90D9"));

        let deserialized: Group = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, group.id);
        assert_eq!(deserialized.name, group.name);
        assert_eq!(deserialized.color, group.color);
    }

    #[test]
    fn test_group_with_servers() {
        let group = Group::new("Production".to_string(), "#D0021B".to_string());
        let servers = vec![
            ServerReference {
                id: "srv-1".to_string(),
                name: "Server 1".to_string(),
                host: "192.168.1.1".to_string(),
                port: 22,
            },
            ServerReference {
                id: "srv-2".to_string(),
                name: "Server 2".to_string(),
                host: "192.168.1.2".to_string(),
                port: 22,
            },
        ];

        let group_with_servers = GroupWithServers {
            group: group.clone(),
            servers: servers.clone(),
        };

        assert_eq!(group_with_servers.group.name, "Production");
        assert_eq!(group_with_servers.servers.len(), 2);
        assert_eq!(group_with_servers.servers[0].name, "Server 1");
    }

    #[test]
    fn test_group_stats() {
        let group = Group::new("Development".to_string(), "#4A90D9".to_string());
        let stats = GroupStats {
            group,
            server_count: 5,
        };

        assert_eq!(stats.server_count, 5);
        assert_eq!(stats.group.name, "Development");
    }

    #[test]
    fn test_create_group_request() {
        let json = r##"{"name": "Staging", "color": "#F5A623"}"##;
        let request: CreateGroupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Staging");
        assert_eq!(request.color, "#F5A623");
    }

    #[test]
    fn test_update_group_request() {
        let json = r##"{"name": "New Name", "color": "#4A90D9"}"##;
        let request: UpdateGroupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("New Name".to_string()));
        assert_eq!(request.color, Some("#4A90D9".to_string()));

        let partial_json = r#"{"name": "Only Name"}"#;
        let partial: UpdateGroupRequest = serde_json::from_str(partial_json).unwrap();
        assert_eq!(partial.name, Some("Only Name".to_string()));
        assert_eq!(partial.color, None);
    }

    #[test]
    fn test_move_server_request() {
        let json = r#"{"server_id": "srv-123", "target_group_id": "grp-456"}"#;
        let request: MoveServerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.server_id, "srv-123");
        assert_eq!(request.target_group_id, Some("grp-456".to_string()));

        let ungroup_json = r#"{"server_id": "srv-123", "target_group_id": null}"#;
        let ungroup: MoveServerRequest = serde_json::from_str(ungroup_json).unwrap();
        assert_eq!(ungroup.server_id, "srv-123");
        assert_eq!(ungroup.target_group_id, None);
    }
}
