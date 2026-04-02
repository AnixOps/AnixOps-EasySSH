//! User Model
//!
//! This module defines the User domain model for the EasySSH application.
//! In Lite edition, this represents a local user profile.
//! In Standard/Pro editions, this can represent both local and team users.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Validatable, ValidationError, MAX_NAME_LENGTH};

/// Unique identifier for users
pub type UserId = String;

/// User status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// User is active and can log in
    Active,
    /// User is inactive (suspended or disabled)
    Inactive,
    /// User is pending approval/activation
    Pending,
}

impl Default for UserStatus {
    fn default() -> Self {
        UserStatus::Active
    }
}

/// User preferences for UI and behavior
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserPreferences {
    /// UI theme (light/dark/system)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Language code (e.g., "zh-CN", "en-US")
    #[serde(default = "default_language")]
    pub language: String,
    /// Whether to show connection notifications
    #[serde(default = "default_true")]
    pub show_notifications: bool,
    /// Whether to enable sound effects
    #[serde(default = "default_false")]
    pub sound_enabled: bool,
    /// Default terminal font size
    #[serde(default = "default_font_size")]
    pub terminal_font_size: u16,
    /// Whether to auto-save connections
    #[serde(default = "default_true")]
    pub auto_save_connections: bool,
    /// Whether to show connection status in sidebar
    #[serde(default = "default_true")]
    pub show_connection_status: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            language: default_language(),
            show_notifications: true,
            sound_enabled: false,
            terminal_font_size: default_font_size(),
            auto_save_connections: true,
            show_connection_status: true,
        }
    }
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "zh-CN".to_string()
}

fn default_font_size() -> u16 {
    14
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    /// Display name
    pub display_name: String,
    /// User's email address
    pub email: Option<String>,
    /// Avatar URL or data URI
    pub avatar: Option<String>,
    /// User's organization/company
    pub organization: Option<String>,
    /// User's role/title
    pub role: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "User".to_string(),
            email: None,
            avatar: None,
            organization: None,
            role: None,
        }
    }
}

/// User domain model
///
/// Represents a user in the EasySSH application.
/// In Lite edition, this is always a local user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier (UUID)
    pub id: UserId,
    /// Username (login name)
    pub username: String,
    /// User status
    #[serde(default)]
    pub status: UserStatus,
    /// User profile information
    #[serde(default)]
    pub profile: UserProfile,
    /// User preferences
    #[serde(default)]
    pub preferences: UserPreferences,
    /// Whether this is a local user (true for Lite edition)
    #[serde(default = "default_true")]
    pub is_local: bool,
    /// Team ID if user belongs to a team (Pro edition)
    pub team_id: Option<String>,
    /// User roles (RBAC for Pro edition)
    #[serde(default)]
    pub roles: Vec<String>,
    /// Last login timestamp
    pub last_login_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version for migrations
    #[serde(default)]
    pub schema_version: u32,
}

impl User {
    /// Create a new local user with generated ID
    pub fn new_local(username: String, display_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            status: UserStatus::Active,
            profile: UserProfile {
                display_name,
                email: None,
                avatar: None,
                organization: None,
                role: None,
            },
            preferences: UserPreferences::default(),
            is_local: true,
            team_id: None,
            roles: vec![],
            last_login_at: None,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Create a user with specific ID (for loading from database)
    pub fn with_id(
        id: UserId,
        username: String,
        display_name: String,
        is_local: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            username,
            status: UserStatus::Active,
            profile: UserProfile {
                display_name,
                email: None,
                avatar: None,
                organization: None,
                role: None,
            },
            preferences: UserPreferences::default(),
            is_local,
            team_id: None,
            roles: vec![],
            last_login_at: None,
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, UserStatus::Active)
    }

    /// Update the user and refresh the updated_at timestamp
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Record a login event
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Get display name (falling back to username)
    pub fn display_name(&self) -> &str {
        if self.profile.display_name.is_empty() {
            &self.username
        } else {
            &self.profile.display_name
        }
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Add a role to the user
    pub fn add_role(&mut self, role: String) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
        self.updated_at = Utc::now();
    }

    /// Remove a role from the user
    pub fn remove_role(&mut self, role: &str) {
        self.roles.retain(|r| r != role);
        self.updated_at = Utc::now();
    }

    /// Update preferences
    pub fn update_preferences<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserPreferences),
    {
        f(&mut self.preferences);
        self.updated_at = Utc::now();
    }

    /// Update profile
    pub fn update_profile<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserProfile),
    {
        f(&mut self.profile);
        self.updated_at = Utc::now();
    }
}

impl Validatable for User {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate username
        if self.username.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "username".to_string(),
                message: "Username cannot be empty".to_string(),
            });
        }
        if self.username.len() > MAX_NAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "username".to_string(),
                message: format!("Username too long (max {} characters)", MAX_NAME_LENGTH),
            });
        }

        // Validate display name in profile
        if self.profile.display_name.len() > MAX_NAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "profile.display_name".to_string(),
                message: format!("Display name too long (max {} characters)", MAX_NAME_LENGTH),
            });
        }

        // Validate email if present
        if let Some(ref email) = self.profile.email {
            if !email.is_empty() && !email.contains('@') {
                return Err(ValidationError::InvalidFormat {
                    field: "profile.email".to_string(),
                    expected: "valid email address".to_string(),
                });
            }
        }

        // Validate terminal font size
        if self.preferences.terminal_font_size < 8 || self.preferences.terminal_font_size > 72 {
            return Err(ValidationError::OutOfRange {
                field: "preferences.terminal_font_size".to_string(),
                min: 8,
                max: 72,
                actual: self.preferences.terminal_font_size as i64,
            });
        }

        Ok(())
    }
}

/// Local user model (simplified version for Lite edition)
///
/// This is a lightweight user model used when no team features are needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    /// Unique identifier
    pub id: UserId,
    /// Display name
    pub display_name: String,
    /// User preferences
    #[serde(default)]
    pub preferences: UserPreferences,
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Schema version
    #[serde(default)]
    pub schema_version: u32,
}

impl LocalUser {
    /// Create a new local user
    pub fn new(display_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            display_name,
            preferences: UserPreferences::default(),
            created_at: now,
            updated_at: now,
            schema_version: 1,
        }
    }

    /// Update display name
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
        self.updated_at = Utc::now();
    }

    /// Update preferences
    pub fn update_preferences<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserPreferences),
    {
        f(&mut self.preferences);
        self.updated_at = Utc::now();
    }
}

impl Validatable for LocalUser {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.display_name.trim().is_empty() {
            return Err(ValidationError::InvalidField {
                field: "display_name".to_string(),
                message: "Display name cannot be empty".to_string(),
            });
        }
        if self.display_name.len() > MAX_NAME_LENGTH {
            return Err(ValidationError::InvalidField {
                field: "display_name".to_string(),
                message: format!("Display name too long (max {} characters)", MAX_NAME_LENGTH),
            });
        }

        if self.preferences.terminal_font_size < 8 || self.preferences.terminal_font_size > 72 {
            return Err(ValidationError::OutOfRange {
                field: "preferences.terminal_font_size".to_string(),
                min: 8,
                max: 72,
                actual: self.preferences.terminal_font_size as i64,
            });
        }

        Ok(())
    }
}

/// DTO for creating a new user
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserDto {
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub is_local: Option<bool>,
}

/// DTO for updating a user
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserDto {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub status: Option<UserStatus>,
    pub preferences: Option<UserPreferences>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new_local() {
        let user = User::new_local("john".to_string(), "John Doe".to_string());
        assert_eq!(user.username, "john");
        assert_eq!(user.profile.display_name, "John Doe");
        assert!(user.is_local);
        assert!(user.is_active());
        assert!(user.last_login_at.is_none());
    }

    #[test]
    fn test_user_with_id() {
        let user = User::with_id(
            "user-123".to_string(),
            "jane".to_string(),
            "Jane Doe".to_string(),
            true,
        );
        assert_eq!(user.id, "user-123");
        assert_eq!(user.username, "jane");
    }

    #[test]
    fn test_user_validation_valid() {
        let user = User::new_local("testuser".to_string(), "Test User".to_string());
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_user_validation_empty_username() {
        let user = User::new_local("".to_string(), "Test".to_string());
        assert!(matches!(user.validate(), Err(ValidationError::InvalidField { field, .. }) if field == "username"));
    }

    #[test]
    fn test_user_validation_long_username() {
        let user = User::new_local("a".repeat(101), "Test".to_string());
        assert!(matches!(user.validate(), Err(ValidationError::InvalidField { field, .. }) if field == "username"));
    }

    #[test]
    fn test_user_validation_invalid_email() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.profile.email = Some("invalid-email".to_string());
        assert!(matches!(user.validate(), Err(ValidationError::InvalidFormat { field, .. }) if field == "profile.email"));
    }

    #[test]
    fn test_user_validation_font_size_range() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.preferences.terminal_font_size = 5;
        assert!(matches!(user.validate(), Err(ValidationError::OutOfRange { field, .. }) if field == "preferences.terminal_font_size"));

        user.preferences.terminal_font_size = 80;
        assert!(matches!(user.validate(), Err(ValidationError::OutOfRange { field, .. }) if field == "preferences.terminal_font_size"));
    }

    #[test]
    fn test_user_record_login() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(user.last_login_at.is_none());

        user.record_login();
        assert!(user.last_login_at.is_some());
    }

    #[test]
    fn test_user_roles() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(!user.has_role("admin"));

        user.add_role("admin".to_string());
        assert!(user.has_role("admin"));

        user.remove_role("admin");
        assert!(!user.has_role("admin"));

        // Adding duplicate should not create duplicate
        user.add_role("admin".to_string());
        user.add_role("admin".to_string());
        assert_eq!(user.roles.len(), 1);
    }

    #[test]
    fn test_user_display_name() {
        let mut user = User::new_local("test".to_string(), "Test User".to_string());
        assert_eq!(user.display_name(), "Test User");

        user.profile.display_name = "".to_string();
        assert_eq!(user.display_name(), "test");
    }

    #[test]
    fn test_user_update() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        let old_updated = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.update(|u| {
            u.profile.display_name = "Updated".to_string();
        });

        assert_eq!(user.profile.display_name, "Updated");
        assert!(user.updated_at > old_updated);
    }

    #[test]
    fn test_user_status() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(user.is_active());

        user.status = UserStatus::Inactive;
        assert!(!user.is_active());

        user.status = UserStatus::Pending;
        assert!(!user.is_active());
    }

    #[test]
    fn test_local_user_new() {
        let user = LocalUser::new("My Name".to_string());
        assert_eq!(user.display_name, "My Name");
        assert_eq!(user.schema_version, 1);
    }

    #[test]
    fn test_local_user_validation() {
        let user = LocalUser::new("Test".to_string());
        assert!(user.validate().is_ok());

        let empty = LocalUser::new("".to_string());
        assert!(empty.validate().is_err());

        let long = LocalUser::new("a".repeat(101));
        assert!(long.validate().is_err());
    }

    #[test]
    fn test_user_preferences_default() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.theme, "system");
        assert_eq!(prefs.language, "zh-CN");
        assert!(prefs.show_notifications);
        assert!(!prefs.sound_enabled);
        assert_eq!(prefs.terminal_font_size, 14);
        assert!(prefs.auto_save_connections);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new_local("test".to_string(), "Test".to_string());
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test"));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, "test");
        assert_eq!(deserialized.profile.display_name, "Test");
    }

    #[test]
    fn test_create_user_dto() {
        let json = r##"{"username": "john", "display_name": "John Doe", "email": "john@example.com"}"##;
        let dto: CreateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.username, "john");
        assert_eq!(dto.display_name, "John Doe");
        assert_eq!(dto.email, Some("john@example.com".to_string()));
        assert_eq!(dto.is_local, None);
    }

    #[test]
    fn test_update_user_dto() {
        let json = r##"{"display_name": "New Name", "status": "inactive"}"##;
        let dto: UpdateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.display_name, Some("New Name".to_string()));
        assert_eq!(dto.status, Some(UserStatus::Inactive));
    }
}
