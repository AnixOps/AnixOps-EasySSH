//! User Model
//!
//! This module defines the User domain model for the EasySSH application.
//! In Lite edition, this represents a local user profile.
//! In Standard/Pro editions, this can represent both local and team users.
//!
//! # Examples
//!
//! ```
//! use easyssh_core::models::{User, Validatable};
//!
//! let user = User::new_local("jdoe".to_string(), "John Doe".to_string());
//! assert!(user.validate().is_ok());
//! assert_eq!(user.username, "jdoe");
//! assert!(user.is_active());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Validatable, ValidationError, MAX_NAME_LENGTH};
use crate::models::validation::is_valid_email;

/// Unique identifier for users
pub type UserId = String;

/// User account status
///
/// Controls whether a user can log in and what operations they can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// User is active and can log in normally
    #[default]
    Active,
    /// User is inactive (suspended or disabled)
    /// Cannot log in until reactivated by an administrator.
    Inactive,
    /// User is pending approval/activation
    /// Account created but not yet confirmed/approved.
    Pending,
}

impl UserStatus {
    /// Check if this status allows login
    pub fn can_login(&self) -> bool {
        matches!(self, UserStatus::Active)
    }

    /// Get display name for this status
    pub fn display_name(&self) -> &'static str {
        match self {
            UserStatus::Active => "Active",
            UserStatus::Inactive => "Inactive",
            UserStatus::Pending => "Pending",
        }
    }
}

/// User preferences for UI and behavior
///
/// These settings control the user experience and can be modified
/// by the user through the application settings.
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
    /// Default terminal font size (8-72)
    #[serde(default = "default_font_size")]
    pub terminal_font_size: u16,
    /// Whether to auto-save connections
    #[serde(default = "default_true")]
    pub auto_save_connections: bool,
    /// Whether to show connection status in sidebar
    #[serde(default = "default_true")]
    pub show_connection_status: bool,
    /// Whether to show server groups expanded by default
    #[serde(default = "default_true")]
    pub expand_groups_by_default: bool,
    /// Whether to confirm before deleting servers
    #[serde(default = "default_true")]
    pub confirm_deletions: bool,
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
            expand_groups_by_default: true,
            confirm_deletions: true,
        }
    }
}

impl Validatable for UserPreferences {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate theme
        let valid_themes = ["light", "dark", "system"];
        if !valid_themes.contains(&self.theme.as_str()) {
            errors.push(ValidationError::invalid_field(
                "theme",
                format!("Theme must be one of: {:?}", valid_themes),
            ));
        }

        // Validate language (basic check for format)
        if self.language.is_empty() || !self.language.contains('-') {
            errors.push(ValidationError::invalid_format(
                "language",
                "valid language code (e.g., 'zh-CN', 'en-US')",
            ));
        }

        // Validate font size
        if self.terminal_font_size < 8 || self.terminal_font_size > 72 {
            errors.push(ValidationError::out_of_range(
                "terminal_font_size",
                8,
                72,
                self.terminal_font_size as i64,
            ));
        }

        ValidationError::combine(errors)
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
///
/// Contains personal information about the user that can be
/// displayed in the UI and used for team management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    /// Display name (shown in UI instead of username)
    #[serde(default = "default_display_name")]
    pub display_name: String,
    /// User's email address
    pub email: Option<String>,
    /// Avatar URL or data URI
    pub avatar: Option<String>,
    /// User's organization/company
    pub organization: Option<String>,
    /// User's role/title
    pub role: Option<String>,
    /// User's timezone
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: default_display_name(),
            email: None,
            avatar: None,
            organization: None,
            role: None,
            timezone: default_timezone(),
        }
    }
}

impl Validatable for UserProfile {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate display name
        if self.display_name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "display_name",
                "Display name cannot be empty",
            ));
        } else if self.display_name.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "display_name",
                format!("Display name too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        // Validate email if present
        if let Some(ref email) = self.email {
            if !email.is_empty() && !is_valid_email(email) {
                errors.push(ValidationError::invalid_format(
                    "email",
                    "valid email address",
                ));
            }
        }

        ValidationError::combine(errors)
    }
}

fn default_display_name() -> String {
    "User".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// User domain model
///
/// Represents a user in the EasySSH application.
/// In Lite edition, this is always a local user.
/// In Pro edition, this can be a team user with RBAC roles.
///
/// # Fields
///
/// * `id` - Unique identifier (UUID v4)
/// * `username` - Login username (unique)
/// * `status` - Account status (active/inactive/pending)
/// * `profile` - Personal information
/// * `preferences` - UI and behavior settings
/// * `is_local` - Whether this is a local account
/// * `team_id` - Team membership (Pro edition)
/// * `roles` - RBAC roles (Pro edition)
/// * `last_login_at` - Last successful login timestamp
/// * `created_at` - Account creation timestamp
/// * `updated_at` - Last modification timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier (UUID)
    pub id: UserId,
    /// Username (login name, unique)
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
    ///
    /// # Arguments
    /// * `username` - Login username
    /// * `display_name` - Display name for the UI
    ///
    /// # Example
    ///
    /// ```
    /// use easyssh_core::models::User;
    ///
    /// let user = User::new_local("jdoe".to_string(), "John Doe".to_string());
    /// assert_eq!(user.username, "jdoe");
    /// assert!(user.is_local);
    /// assert!(user.is_active());
    /// ```
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
                timezone: default_timezone(),
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
    ///
    /// # Arguments
    /// * `id` - User ID
    /// * `username` - Login username
    /// * `display_name` - Display name
    /// * `is_local` - Whether this is a local account
    pub fn with_id(id: UserId, username: String, display_name: String, is_local: bool) -> Self {
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
                timezone: default_timezone(),
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
    ///
    /// Active users can log in and use the application.
    pub fn is_active(&self) -> bool {
        self.status.can_login()
    }

    /// Check if user can log in
    ///
    /// Alias for `is_active()`.
    pub fn can_login(&self) -> bool {
        self.is_active()
    }

    /// Update the user and refresh the updated_at timestamp
    ///
    /// # Arguments
    /// * `f` - Closure that performs the modifications
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        f(self);
        self.updated_at = Utc::now();
    }

    /// Record a login event
    ///
    /// Updates last_login_at and updated_at timestamps.
    pub fn record_login(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Get display name (falling back to username)
    ///
    /// Returns the display name if set, otherwise the username.
    pub fn display_name(&self) -> &str {
        if self.profile.display_name.is_empty() || self.profile.display_name == "User" {
            &self.username
        } else {
            &self.profile.display_name
        }
    }

    /// Get full display label (Name `<username>`)
    pub fn display_label(&self) -> String {
        if self.display_name() != self.username {
            format!("{} ({})", self.display_name(), self.username)
        } else {
            self.username.clone()
        }
    }

    /// Check if user has a specific role
    ///
    /// Used for RBAC in Pro edition.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Add a role to the user
    ///
    /// # Arguments
    /// * `role` - Role name to add
    ///
    /// Does nothing if the user already has the role.
    pub fn add_role(&mut self, role: String) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
        self.updated_at = Utc::now();
    }

    /// Remove a role from the user
    ///
    /// # Arguments
    /// * `role` - Role name to remove
    pub fn remove_role(&mut self, role: &str) {
        self.roles.retain(|r| r != role);
        self.updated_at = Utc::now();
    }

    /// Update preferences
    ///
    /// # Arguments
    /// * `f` - Closure that modifies preferences
    pub fn update_preferences<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserPreferences),
    {
        f(&mut self.preferences);
        self.updated_at = Utc::now();
    }

    /// Update profile
    ///
    /// # Arguments
    /// * `f` - Closure that modifies profile
    pub fn update_profile<F>(&mut self, f: F)
    where
        F: FnOnce(&mut UserProfile),
    {
        f(&mut self.profile);
        self.updated_at = Utc::now();
    }

    /// Activate the user account
    ///
    /// Changes status from Pending or Inactive to Active.
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Deactivate the user account
    ///
    /// Changes status to Inactive. User will not be able to log in.
    pub fn deactivate(&mut self) {
        self.status = UserStatus::Inactive;
        self.updated_at = Utc::now();
    }

    /// Clone without sensitive data
    ///
    /// Creates a copy suitable for logging or API responses.
    pub fn clone_redacted(&self) -> Self {
        Self {
            id: self.id.clone(),
            username: self.username.clone(),
            status: self.status,
            profile: self.profile.clone(),
            preferences: self.preferences.clone(),
            is_local: self.is_local,
            team_id: self.team_id.clone(),
            roles: self.roles.clone(),
            last_login_at: self.last_login_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            schema_version: self.schema_version,
        }
    }
}

impl Validatable for User {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        // Validate username
        if self.username.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "username",
                "Username cannot be empty",
            ));
        } else if self.username.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "username",
                format!("Username too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        // Validate profile
        if let Err(e) = self.profile.validate() {
            errors.push(e);
        }

        // Validate preferences
        if let Err(e) = self.preferences.validate() {
            errors.push(e);
        }

        // Validate roles (check for empty role names)
        for role in &self.roles {
            if role.trim().is_empty() {
                errors.push(ValidationError::invalid_field(
                    "roles",
                    "Role names cannot be empty",
                ));
                break;
            }
        }

        ValidationError::combine(errors)
    }
}

/// Local user model (simplified version for Lite edition)
///
/// This is a lightweight user model used when no team features are needed.
/// It contains only the essential fields for a single-user setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    /// Unique identifier
    pub id: UserId,
    /// Display name
    #[serde(default = "default_display_name")]
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
    ///
    /// # Arguments
    /// * `display_name` - The user's display name
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
    ///
    /// # Arguments
    /// * `name` - New display name
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
        self.updated_at = Utc::now();
    }

    /// Update preferences
    ///
    /// # Arguments
    /// * `f` - Closure that modifies preferences
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
        let mut errors = Vec::new();

        if self.display_name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "display_name",
                "Display name cannot be empty",
            ));
        } else if self.display_name.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "display_name",
                format!("Display name too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        if let Err(e) = self.preferences.validate() {
            errors.push(e);
        }

        ValidationError::combine(errors)
    }
}

/// DTO for creating a new user
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserDto {
    /// Username (required, unique)
    pub username: String,
    /// Display name
    pub display_name: String,
    /// Email address (optional)
    pub email: Option<String>,
    /// Whether this is a local account (default: true)
    #[serde(default = "default_true")]
    pub is_local: bool,
    /// Initial roles (optional)
    #[serde(default)]
    pub roles: Vec<String>,
}

impl Validatable for CreateUserDto {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        if self.username.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "username",
                "Username cannot be empty",
            ));
        } else if self.username.len() > MAX_NAME_LENGTH {
            errors.push(ValidationError::invalid_field(
                "username",
                format!("Username too long (max {} characters)", MAX_NAME_LENGTH),
            ));
        }

        if self.display_name.trim().is_empty() {
            errors.push(ValidationError::invalid_field(
                "display_name",
                "Display name cannot be empty",
            ));
        }

        if let Some(ref email) = self.email {
            if !email.is_empty() && !is_valid_email(email) {
                errors.push(ValidationError::invalid_format(
                    "email",
                    "valid email address",
                ));
            }
        }

        ValidationError::combine(errors)
    }
}

/// DTO for updating a user
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserDto {
    /// New display name
    pub display_name: Option<String>,
    /// New email
    pub email: Option<String>,
    /// New status
    pub status: Option<UserStatus>,
    /// New preferences (replaces existing)
    pub preferences: Option<UserPreferences>,
    /// New profile (partial update)
    pub profile: Option<UserProfile>,
}

impl Validatable for UpdateUserDto {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        if let Some(ref display_name) = self.display_name {
            if display_name.trim().is_empty() {
                errors.push(ValidationError::invalid_field(
                    "display_name",
                    "Display name cannot be empty",
                ));
            } else if display_name.len() > MAX_NAME_LENGTH {
                errors.push(ValidationError::invalid_field(
                    "display_name",
                    format!("Display name too long (max {} characters)", MAX_NAME_LENGTH),
                ));
            }
        }

        if let Some(ref email) = self.email {
            if !email.is_empty() && !is_valid_email(email) {
                errors.push(ValidationError::invalid_format(
                    "email",
                    "valid email address",
                ));
            }
        }

        if let Some(ref preferences) = self.preferences {
            if let Err(e) = preferences.validate() {
                errors.push(e);
            }
        }

        if let Some(ref profile) = self.profile {
            if let Err(e) = profile.validate() {
                errors.push(e);
            }
        }

        ValidationError::combine(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new_local() {
        let user = User::new_local("jdoe".to_string(), "John Doe".to_string());
        assert_eq!(user.username, "jdoe");
        assert_eq!(user.profile.display_name, "John Doe");
        assert!(user.is_local);
        assert!(user.is_active());
        assert!(user.can_login());
        assert!(user.last_login_at.is_none());
        assert_eq!(user.schema_version, 1);
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
        assert!(user.is_local);
    }

    #[test]
    fn test_user_validation_valid() {
        let user = User::new_local("testuser".to_string(), "Test User".to_string());
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_user_validation_empty_username() {
        let user = User::new_local("".to_string(), "Test".to_string());
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
    }

    #[test]
    fn test_user_validation_long_username() {
        let user = User::new_local("a".repeat(101), "Test".to_string());
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("username"));
    }

    #[test]
    fn test_user_validation_invalid_email() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.profile.email = Some("invalid-email".to_string());
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("email"));
    }

    #[test]
    fn test_user_validation_valid_email() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.profile.email = Some("user@example.com".to_string());
        assert!(user.validate().is_ok());
    }

    #[test]
    fn test_user_validation_invalid_theme() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.preferences.theme = "invalid".to_string();
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("theme"));
    }

    #[test]
    fn test_user_validation_font_size_range() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.preferences.terminal_font_size = 5;
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("terminal_font_size"));

        user.preferences.terminal_font_size = 80;
        let result = user.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field(), Some("terminal_font_size"));
    }

    #[test]
    fn test_user_record_login() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(user.last_login_at.is_none());

        user.record_login();
        assert!(user.last_login_at.is_some());
        assert!(user.last_login_at.unwrap() <= Utc::now());
    }

    #[test]
    fn test_user_roles() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(!user.has_role("admin"));
        assert!(!user.has_any_role(&["admin", "superuser"]));

        user.add_role("admin".to_string());
        assert!(user.has_role("admin"));
        assert!(user.has_any_role(&["admin", "superuser"]));
        assert!(!user.has_any_role(&["superuser", "owner"]));

        user.remove_role("admin");
        assert!(!user.has_role("admin"));

        // Adding duplicate should not create duplicate
        user.add_role("admin".to_string());
        user.add_role("admin".to_string());
        assert_eq!(user.roles.len(), 1);
    }

    #[test]
    fn test_user_display_name() {
        let mut user = User::new_local("jdoe".to_string(), "John Doe".to_string());
        assert_eq!(user.display_name(), "John Doe");

        user.profile.display_name = "".to_string();
        assert_eq!(user.display_name(), "jdoe");

        user.profile.display_name = "User".to_string();
        assert_eq!(user.display_name(), "jdoe"); // Falls back for default
    }

    #[test]
    fn test_user_display_label() {
        let user = User::new_local("jdoe".to_string(), "John Doe".to_string());
        assert_eq!(user.display_label(), "John Doe (jdoe)");

        let user_same = User::new_local("admin".to_string(), "admin".to_string());
        assert_eq!(user_same.display_label(), "admin");
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
    fn test_user_update_preferences() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        let old_updated = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.update_preferences(|p| {
            p.theme = "dark".to_string();
        });

        assert_eq!(user.preferences.theme, "dark");
        assert!(user.updated_at > old_updated);
    }

    #[test]
    fn test_user_update_profile() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        let old_updated = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.update_profile(|p| {
            p.email = Some("test@example.com".to_string());
        });

        assert_eq!(user.profile.email, Some("test@example.com".to_string()));
        assert!(user.updated_at > old_updated);
    }

    #[test]
    fn test_user_status() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        assert!(user.is_active());
        assert!(user.status.can_login());
        assert_eq!(user.status.display_name(), "Active");

        user.status = UserStatus::Inactive;
        assert!(!user.is_active());
        assert!(!user.can_login());
        assert_eq!(user.status.display_name(), "Inactive");

        user.status = UserStatus::Pending;
        assert!(!user.is_active());
        assert!(!user.can_login());
        assert_eq!(user.status.display_name(), "Pending");
    }

    #[test]
    fn test_user_activate_deactivate() {
        let mut user = User::new_local("test".to_string(), "Test".to_string());
        user.status = UserStatus::Pending;

        let old_updated = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.activate();
        assert!(matches!(user.status, UserStatus::Active));
        assert!(user.updated_at > old_updated);

        user.deactivate();
        assert!(matches!(user.status, UserStatus::Inactive));
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
    fn test_local_user_set_display_name() {
        let mut user = LocalUser::new("Original".to_string());
        let old_updated = user.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        user.set_display_name("New Name".to_string());
        assert_eq!(user.display_name, "New Name");
        assert!(user.updated_at > old_updated);
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
        assert!(prefs.show_connection_status);
        assert!(prefs.expand_groups_by_default);
        assert!(prefs.confirm_deletions);
    }

    #[test]
    fn test_user_preferences_validation() {
        let valid = UserPreferences::default();
        assert!(valid.validate().is_ok());

        let invalid_theme = UserPreferences {
            theme: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_theme.validate().is_err());

        let invalid_language = UserPreferences {
            language: "invalid".to_string(),
            ..Default::default()
        };
        assert!(invalid_language.validate().is_err());

        let invalid_font_size = UserPreferences {
            terminal_font_size: 5,
            ..Default::default()
        };
        assert!(invalid_font_size.validate().is_err());
    }

    #[test]
    fn test_user_profile_default() {
        let profile = UserProfile::default();
        assert_eq!(profile.display_name, "User");
        assert!(profile.email.is_none());
        assert!(profile.avatar.is_none());
        assert_eq!(profile.timezone, "UTC");
    }

    #[test]
    fn test_user_profile_validation() {
        let valid = UserProfile {
            display_name: "John".to_string(),
            email: Some("john@example.com".to_string()),
            ..Default::default()
        };
        assert!(valid.validate().is_ok());

        let empty_name = UserProfile {
            display_name: "".to_string(),
            ..Default::default()
        };
        assert!(empty_name.validate().is_err());

        let long_name = UserProfile {
            display_name: "a".repeat(101),
            ..Default::default()
        };
        assert!(long_name.validate().is_err());

        let invalid_email = UserProfile {
            display_name: "John".to_string(),
            email: Some("invalid".to_string()),
            ..Default::default()
        };
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new_local("test".to_string(), "Test".to_string());
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test"));
        assert!(json.contains("active"));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, "test");
        assert_eq!(deserialized.profile.display_name, "Test");
    }

    #[test]
    fn test_create_user_dto() {
        let json = r##"{
            "username": "jdoe",
            "display_name": "John Doe",
            "email": "john@example.com",
            "is_local": true,
            "roles": ["admin", "user"]
        }"##;
        let dto: CreateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.username, "jdoe");
        assert_eq!(dto.display_name, "John Doe");
        assert_eq!(dto.email, Some("john@example.com".to_string()));
        assert!(dto.is_local);
        assert_eq!(dto.roles, vec!["admin".to_string(), "user".to_string()]);
        assert!(dto.validate().is_ok());
    }

    #[test]
    fn test_create_user_dto_validation() {
        let invalid = CreateUserDto {
            username: "".to_string(),
            display_name: "Test".to_string(),
            email: None,
            is_local: true,
            roles: vec![],
        };
        assert!(invalid.validate().is_err());

        let invalid_email = CreateUserDto {
            username: "test".to_string(),
            display_name: "Test".to_string(),
            email: Some("invalid".to_string()),
            is_local: true,
            roles: vec![],
        };
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_update_user_dto() {
        let json = r##"{
            "display_name": "New Name",
            "status": "inactive",
            "preferences": {
                "theme": "dark",
                "language": "en-US"
            }
        }"##;
        let dto: UpdateUserDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.display_name, Some("New Name".to_string()));
        assert_eq!(dto.status, Some(UserStatus::Inactive));
        assert!(dto.preferences.is_some());
    }

    #[test]
    fn test_update_user_dto_validation() {
        let invalid = UpdateUserDto {
            display_name: Some("".to_string()),
            email: None,
            status: None,
            preferences: None,
            profile: None,
        };
        assert!(invalid.validate().is_err());

        let invalid_email = UpdateUserDto {
            display_name: None,
            email: Some("invalid".to_string()),
            status: None,
            preferences: None,
            profile: None,
        };
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_user_clone_redacted() {
        let user = User::new_local("test".to_string(), "Test".to_string());
        let redacted = user.clone_redacted();
        assert_eq!(redacted.id, user.id);
        assert_eq!(redacted.username, user.username);
        assert_eq!(redacted.status, user.status);
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_NAME_LENGTH, 100);
    }
}
