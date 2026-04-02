//! Group models for EasySSH
//!
//! This module provides data structures for server group management.
//! Groups are used to organize servers in a flat (non-nested) structure for the Lite edition.
//!
//! # Examples
//!
//! ```
//! use easyssh_core::models::{Group, Validatable};
//!
//! let group = Group::new("Production".to_string(), "#D0021B".to_string());
//! assert!(group.validate().is_ok());
//! assert_eq!(group.name, "Production");
//! ```

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

/// Maximum number of groups per user (to prevent abuse)
pub const MAX_GROUPS_PER_USER: usize = 100;

/// Validates a hex color string
///
/// # Arguments
/// * `color` - The color string to validate
///
/// # Returns
/// * `true` if valid (e.g., "#4A90D9", "#4a90d9", "#FFF")
/// * `false` otherwise
///
/// # Examples
///
/// ```
/// use easyssh_core::models::group::is_valid_hex_color;
///
/// assert!(is_valid_hex_color("#4A90D9"));
/// assert!(is_valid_hex_color("#FFF"));
///
/// assert!(!is_valid_hex_color("4A90D9"));  // Missing #
/// assert!(!is_valid_hex_color("#GGGGGG")); // Invalid hex
/// ```
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
///
/// # Fields
///
/// * `id` - Unique identifier (UUID v4)
/// * `name` - Display name of the group
/// * `color` - Hex color code for UI display (e.g., "#4A90D9")
/// * `created_at` - Creation timestamp
/// * `updated_at` - Last modification timestamp
/// * `schema_version` - Data schema version for migrations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    /// Unique identifier for the group
    pub id: GroupId,
    /// Display name of the group
    pub name: String,
    /// Hex color code for UI display (e.g., "#4A90D9")
    pub color: String,
    /// When the group was created
    #[serde(default = "Utc::now")]
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
    ///
    /// ```
    /// use easyssh_core::models::group::Group;
    ///
    /// let group = Group::new("Production".to_string(), "#FF5733".to_string());
    /// assert_eq!(group.name, "Production");
    /// assert!(!group.id.is_empty());
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
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::group::{Group, UNGROUPED_ID, UNGROUPED_NAME, UNGROUPED_COLOR};
    ///
    /// let ungrouped = Group::with_id(
    ///     UNGROUPED_ID.to_string(),
    ///     UNGROUPED_NAME.to_string(),
    ///     UNGROUPED_COLOR.to_string(),
    /// );
    /// assert!(ungrouped.is_ungrouped());
    /// ```
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

    /// Create the default "ungrouped" system group
    ///
    /// This is a special group that cannot be deleted and serves as the default
    /// location for servers that are not assigned to any user-created group.
    pub fn ungrouped() -> Self {
        Self::with_id(
            UNGROUPED_ID.to_string(),
            UNGROUPED_NAME.to_string(),
            UNGROUPED_COLOR.to_string(),
        )
    }

    /// Checks if this is the default "ungrouped" system group
    pub fn is_ungrouped(&self) -> bool {
        self.id == UNGROUPED_ID
    }

    /// Checks if this group can be deleted
    ///
    /// The ungrouped system group cannot be deleted.
    pub fn can_delete(&self) -> bool {
        !self.is_ungrouped()
    }

    /// Checks if this group can be renamed
    ///
    /// The ungrouped system group cannot be renamed.
    pub fn can_rename(&self) -> bool {
        !self.is_ungrouped()
    }

    /// Update the group and refresh the updated_at timestamp
    ///
    /// # Arguments
    /// * `f` - Closure that performs the modifications
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::group::Group;
    ///
    /// let mut group = Group::new("Dev".to_string(), "#4A90D9".to_string());
    /// group.update(|g| {
    ///     g.name = "Development".to_string();
    /// });
    /// assert_eq!(group.name, "Development");
    /// ```
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Get a display label for UI lists
    pub fn display_label(&self) -> String {
        if self.is_ungrouped() {
            self.name.to_string()
        } else {
            format!("{} ({})", self.name, self.color)
        }
    }

    /// Get a sort key for ordering groups
    ///
    /// Ungrouped always comes first, then alphabetical by name.
    pub fn sort_key(&self) -> String {
        if self.is_ungrouped() {
            "_".to_string() // Sort first
        } else {
            self.name.to_lowercase()
        }
    }
}

impl Validatable for Group {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate name
        if self.name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "name",
                "Group name cannot be empty",
            ));
        } else if self.name.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "name",
                format!("Group name too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        // Validate color
        if !is_valid_hex_color(&self.color) {
            errors.push(ValidationError::invalid_format(
                "color",
                "valid hex color (e.g., #4A90D9 or #FFF)",
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Preset group definitions for new installations
///
/// These are suggested groups that can be created for new users.
pub const PRESET_GROUPS: &[(&str, &str)] = &[
    ("开发", "#4A90D9"), // Blue
    ("测试", "#F5A623"), // Orange
    ("生产", "#D0021B"), // Red
];

/// Default color palette for group selection
///
/// A curated list of 18 colors suitable for UI display.
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
///
/// Used for displaying groups with their server counts in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupStats {
    /// The group information
    pub group: Group,
    /// Number of servers in this group
    pub server_count: usize,
}

impl GroupStats {
    /// Create new group stats
    pub fn new(group: Group, server_count: usize) -> Self {
        Self {
            group,
            server_count,
        }
    }
}

/// Represents a server reference (minimal info for group display)
///
/// This is a lightweight struct to avoid circular dependencies
/// and reduce data transfer when listing group contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerReference {
    /// Server unique identifier
    pub id: String,
    /// Server display name
    pub name: String,
    /// Server host
    pub host: String,
    /// Server port
    pub port: i64,
}

/// Group with its associated servers
///
/// Used for displaying a complete group view with all member servers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupWithServers {
    /// The group information
    pub group: Group,
    /// List of servers in this group
    pub servers: Vec<ServerReference>,
}

impl GroupWithServers {
    /// Check if the group contains a specific server
    pub fn contains_server(&self, server_id: &str) -> bool {
        self.servers.iter().any(|s| s.id == server_id)
    }

    /// Get the number of servers in the group
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Check if the group is empty
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }
}

/// Request to create a new group
///
/// Used for API requests to create groups.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroupRequest {
    /// Group name (required)
    pub name: String,
    /// Group color (required, hex format)
    pub color: String,
}

impl Validatable for CreateGroupRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "name",
                "Group name cannot be empty",
            ));
        } else if self.name.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "name",
                format!("Group name too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        if !is_valid_hex_color(&self.color) {
            errors.push(ValidationError::invalid_format(
                "color",
                "valid hex color (e.g., #4A90D9 or #FFF)",
            ));
        }

        ValidationError::combine(errors)
    }
}

/// Request to update an existing group
///
/// All fields are optional, allowing partial updates.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGroupRequest {
    /// New group name
    pub name: Option<String>,
    /// New group color
    pub color: Option<String>,
}

impl Validatable for UpdateGroupRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                errors.push(ValidationError::invalid_field(
                    "name",
                    "Group name cannot be empty",
                ));
            } else if name.len() > MAX_NAME_LENGTH {
                errors.push(ValidationError::invalid_field(
                    "name",
                    format!("Group name too long (max {} characters)", MAX_NAME_LENGTH),
                ));
            }
        }

        if let Some(ref color) = self.color {
            if !is_valid_hex_color(color) {
                errors.push(ValidationError::invalid_format(
                    "color",
                    "valid hex color (e.g., #4A90D9 or #FFF)",
                ));
            }
        }

        ValidationError::combine(errors)
    }
}

/// Request to move a server to a different group
#[derive(Debug, Clone, Deserialize)]
pub struct MoveServerRequest {
    /// Server ID to move
    pub server_id: String,
    /// Target group ID (None = ungroup)
    pub target_group_id: Option<GroupId>,
}

impl Validatable for MoveServerRequest {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.server_id.trim().is_empty() {
            return Err(ValidationError::invalid_field(
                "server_id",
                "Server ID cannot be empty",
            ));
        }
        Ok(())
    }
}

/// Validates a list of groups
///
/// Checks that the number of groups is within limits and there are no duplicates.
pub fn validate_group_list(groups: &[Group]) -> Result<(), ValidationError> {
    if groups.len() > MAX_GROUPS_PER_USER {
        return Err(ValidationError::ConstraintViolation {
            constraint: "max_groups".to_string(),
            message: format!(
                "Too many groups: {} (max: {})",
                groups.len(),
                MAX_GROUPS_PER_USER
            ),
        });
    }

    // Check for duplicate names
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for group in groups {
        let name = group.name.trim().to_lowercase();
        if !names.insert(name) {
            return Err(ValidationError::Duplicate {
                field: "name".to_string(),
                value: group.name.clone(),
            });
        }
    }

    Ok(())
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
        assert_eq!(group.schema_version, 1);
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
        assert_eq!(group.color, "#FF5733");
    }

    #[test]
    fn test_group_ungrouped() {
        let ungrouped = Group::ungrouped();
        assert_eq!(ungrouped.id, UNGROUPED_ID);
        assert_eq!(ungrouped.name, UNGROUPED_NAME);
        assert_eq!(ungrouped.color, UNGROUPED_COLOR);
        assert!(ungrouped.is_ungrouped());
    }

    #[test]
    fn test_is_ungrouped() {
        let ungrouped = Group::ungrouped();
        assert!(ungrouped.is_ungrouped());
        assert!(!ungrouped.can_delete());
        assert!(!ungrouped.can_rename());

        let regular = Group::new("Regular".to_string(), "#4A90D9".to_string());
        assert!(!regular.is_ungrouped());
        assert!(regular.can_delete());
        assert!(regular.can_rename());
    }

    #[test]
    fn test_is_valid_hex_color() {
        assert!(is_valid_hex_color("#4A90D9"));
        assert!(is_valid_hex_color("#4a90d9"));
        assert!(is_valid_hex_color("#FFF"));
        assert!(is_valid_hex_color("#fff"));
        assert!(is_valid_hex_color("#123ABC"));
        assert!(is_valid_hex_color("#000"));
        assert!(is_valid_hex_color("#ffffff"));

        assert!(!is_valid_hex_color("4A90D9")); // Missing #
        assert!(!is_valid_hex_color("#GGGGGG")); // Invalid hex chars
        assert!(!is_valid_hex_color("#4A90D")); // Wrong length (5)
        assert!(!is_valid_hex_color("#4A90D99")); // Wrong length (7)
        assert!(!is_valid_hex_color("")); // Empty
        assert!(!is_valid_hex_color("#")); // Just #
        assert!(!is_valid_hex_color("#12")); // Too short
        assert!(!is_valid_hex_color("#1234567")); // Too long
    }

    #[test]
    fn test_group_validation_valid() {
        let valid_group = Group::new("Test".to_string(), "#4A90D9".to_string());
        assert!(valid_group.validate().is_ok());
    }

    #[test]
    fn test_group_validation_empty_name() {
        let invalid_group = Group::new("".to_string(), "#4A90D9".to_string());
        let result = invalid_group.validate();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.field(), Some("name"));
    }

    #[test]
    fn test_group_validation_whitespace_name() {
        let invalid_group = Group::new("   ".to_string(), "#4A90D9".to_string());
        assert!(invalid_group.validate().is_err());
    }

    #[test]
    fn test_group_validation_long_name() {
        let invalid_group = Group::new("a".repeat(101), "#4A90D9".to_string());
        let result = invalid_group.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("name"));
    }

    #[test]
    fn test_group_validation_invalid_color() {
        let invalid_group = Group::new("Test".to_string(), "invalid".to_string());
        let result = invalid_group.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("color"));
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
    fn test_group_display_label() {
        let group = Group::new("Development".to_string(), "#4A90D9".to_string());
        assert_eq!(group.display_label(), "Development (#4A90D9)");

        let ungrouped = Group::ungrouped();
        assert_eq!(ungrouped.display_label(), UNGROUPED_NAME);
    }

    #[test]
    fn test_group_sort_key() {
        let ungrouped = Group::ungrouped();
        assert_eq!(ungrouped.sort_key(), "_");

        let group = Group::new("Development".to_string(), "#4A90D9".to_string());
        assert_eq!(group.sort_key(), "development");
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
        assert_eq!(PRESET_GROUPS[0].1, "#4A90D9");
        assert_eq!(PRESET_GROUPS[1].0, "测试");
        assert_eq!(PRESET_GROUPS[1].1, "#F5A623");
        assert_eq!(PRESET_GROUPS[2].0, "生产");
        assert_eq!(PRESET_GROUPS[2].1, "#D0021B");

        // All preset colors should be valid
        for (_, color) in PRESET_GROUPS {
            assert!(is_valid_hex_color(color), "Invalid color: {}", color);
        }
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
        assert!(group_with_servers.contains_server("srv-1"));
        assert!(!group_with_servers.contains_server("srv-999"));
        assert_eq!(group_with_servers.server_count(), 2);
        assert!(!group_with_servers.is_empty());
    }

    #[test]
    fn test_group_with_servers_empty() {
        let group = Group::new("Empty".to_string(), "#9CA3AF".to_string());
        let group_with_servers = GroupWithServers {
            group,
            servers: vec![],
        };
        assert!(group_with_servers.is_empty());
        assert_eq!(group_with_servers.server_count(), 0);
    }

    #[test]
    fn test_group_stats() {
        let group = Group::new("Development".to_string(), "#4A90D9".to_string());
        let stats = GroupStats::new(group, 5);

        assert_eq!(stats.server_count, 5);
        assert_eq!(stats.group.name, "Development");
    }

    #[test]
    fn test_create_group_request() {
        let json = r##"{"name": "Staging", "color": "#F5A623"}"##;
        let request: CreateGroupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Staging");
        assert_eq!(request.color, "#F5A623");
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_create_group_request_validation() {
        let valid = CreateGroupRequest {
            name: "Test".to_string(),
            color: "#4A90D9".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_name = CreateGroupRequest {
            name: "".to_string(),
            color: "#4A90D9".to_string(),
        };
        assert!(invalid_name.validate().is_err());

        let invalid_color = CreateGroupRequest {
            name: "Test".to_string(),
            color: "invalid".to_string(),
        };
        assert!(invalid_color.validate().is_err());
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
    fn test_update_group_request_validation() {
        let valid = UpdateGroupRequest {
            name: Some("Test".to_string()),
            color: Some("#4A90D9".to_string()),
        };
        assert!(valid.validate().is_ok());

        let empty_partial = UpdateGroupRequest {
            name: None,
            color: None,
        };
        assert!(empty_partial.validate().is_ok()); // Nothing to validate

        let invalid_name = UpdateGroupRequest {
            name: Some("".to_string()),
            color: None,
        };
        assert!(invalid_name.validate().is_err());

        let invalid_color = UpdateGroupRequest {
            name: None,
            color: Some("invalid".to_string()),
        };
        assert!(invalid_color.validate().is_err());
    }

    #[test]
    fn test_move_server_request() {
        let json = r#"{"server_id": "srv-123", "target_group_id": "grp-456"}"#;
        let request: MoveServerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.server_id, "srv-123");
        assert_eq!(request.target_group_id, Some("grp-456".to_string()));
        assert!(request.validate().is_ok());

        let ungroup_json = r#"{"server_id": "srv-123", "target_group_id": null}"#;
        let ungroup: MoveServerRequest = serde_json::from_str(ungroup_json).unwrap();
        assert_eq!(ungroup.server_id, "srv-123");
        assert_eq!(ungroup.target_group_id, None);
    }

    #[test]
    fn test_move_server_request_validation() {
        let valid = MoveServerRequest {
            server_id: "srv-123".to_string(),
            target_group_id: Some("grp-456".to_string()),
        };
        assert!(valid.validate().is_ok());

        let empty_id = MoveServerRequest {
            server_id: "".to_string(),
            target_group_id: Some("grp-456".to_string()),
        };
        assert!(empty_id.validate().is_err());

        let whitespace_id = MoveServerRequest {
            server_id: "   ".to_string(),
            target_group_id: None,
        };
        assert!(whitespace_id.validate().is_err());
    }

    #[test]
    fn test_validate_group_list() {
        let valid_groups = vec![
            Group::new("Group 1".to_string(), "#4A90D9".to_string()),
            Group::new("Group 2".to_string(), "#F5A623".to_string()),
        ];
        assert!(validate_group_list(&valid_groups).is_ok());

        // Test with too many groups
        let too_many: Vec<Group> = (0..MAX_GROUPS_PER_USER + 1)
            .map(|i| Group::new(format!("Group {}", i), "#4A90D9".to_string()))
            .collect();
        let result = validate_group_list(&too_many);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Too many groups"));

        // Test with duplicate names
        let duplicates = vec![
            Group::new("Same Name".to_string(), "#4A90D9".to_string()),
            Group::new("Same Name".to_string(), "#F5A623".to_string()),
        ];
        let result = validate_group_list(&duplicates);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::Duplicate { .. }));
    }

    #[test]
    fn test_validate_group_list_case_insensitive() {
        // Names that differ only in case should be considered duplicates
        let case_duplicates = vec![
            Group::new("Production".to_string(), "#4A90D9".to_string()),
            Group::new("production".to_string(), "#F5A623".to_string()),
        ];
        let result = validate_group_list(&case_duplicates);
        assert!(result.is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(UNGROUPED_ID, "_ungrouped");
        assert_eq!(UNGROUPED_NAME, "未分组");
        assert_eq!(UNGROUPED_COLOR, "#9CA3AF");
        assert!(is_valid_hex_color(UNGROUPED_COLOR));
        assert_eq!(MAX_GROUPS_PER_USER, 100);
    }
}
