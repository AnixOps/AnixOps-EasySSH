use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub schema_version: u32,
    pub connections: Vec<Connection>,
    pub groups: Vec<Group>,
    pub recent_connection_ids: Vec<String>,
    pub snippets: Vec<CommandSnippet>,
    #[serde(default)]
    pub sessions: Vec<SessionRecord>,
    #[serde(default)]
    pub workspace: Workspace,
    pub theme: Theme,
    #[serde(default)]
    pub display_density: DisplayDensity,
    pub locale: Locale,
    #[serde(default)]
    pub sidebar: SidebarPreferences,
    #[serde(default)]
    pub sync: SyncPreferences,
}

impl AppConfig {
    pub const SCHEMA_VERSION: u32 = 5;

    pub fn new() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            theme: Theme::System,
            locale: Locale::System,
            workspace: Workspace::Hosts,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayDensity {
    Compact,
    #[default]
    Comfortable,
    Large,
}

/// Local-only Git metadata sync configuration. Credentials are intentionally
/// absent: system Git and its configured credential helper own authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct SyncPreferences {
    pub display_name: String,
    pub repository_path: Option<String>,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_snapshot_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ConnectionTarget {
    Alias {
        alias: String,
    },
    Endpoint {
        hostname: String,
        username: Option<String>,
        port: u16,
    },
}

impl Default for ConnectionTarget {
    fn default() -> Self {
        Self::Alias {
            alias: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub target: ConnectionTarget,
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub color: Option<String>,
    pub notes: String,
    pub proxy_jump: Option<String>,
    pub local_forwards: Vec<String>,
    pub remote_forwards: Vec<String>,
    pub dynamic_forwards: Vec<String>,
    pub remote_command: Option<String>,
    pub local_directories: Vec<String>,
    pub remote_directories: Vec<String>,
    pub terminal: TerminalPreferences,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Connection {
    pub fn alias(name: impl Into<String>, alias: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            target: ConnectionTarget::Alias {
                alias: alias.into(),
            },
            group_id: None,
            tags: Vec::new(),
            favorite: false,
            color: None,
            notes: String::new(),
            proxy_jump: None,
            local_forwards: Vec::new(),
            remote_forwards: Vec::new(),
            dynamic_forwards: Vec::new(),
            remote_command: None,
            local_directories: Vec::new(),
            remote_directories: Vec::new(),
            terminal: TerminalPreferences::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// Non-sensitive presentation state persisted alongside connection metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SidebarPreferences {
    pub width: f32,
    pub collapsed_group_ids: Vec<String>,
    pub selected_navigation: SidebarNavigation,
    pub selected_tag: Option<String>,
}

impl Default for SidebarPreferences {
    fn default() -> Self {
        Self {
            width: 300.0,
            collapsed_group_ids: Vec::new(),
            selected_navigation: SidebarNavigation::All,
            selected_tag: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SidebarNavigation {
    #[default]
    All,
    Favorites,
    Recent,
    OpenSshConfig,
    Ungrouped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSnippet {
    pub id: String,
    pub name: String,
    pub content: String,
}

/// A local record of an external terminal launch. It intentionally contains no
/// command line, terminal output, authentication material, or SSH diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionRecord {
    pub id: String,
    pub connection_id: Option<String>,
    pub name: String,
    pub target: String,
    pub launched_at: DateTime<Utc>,
    pub verbose: bool,
    pub launched: bool,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Workspace {
    #[default]
    Hosts,
    Snippets,
    Forwarding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalPreferences {
    pub font_size: f32,
    pub scrollback_lines: usize,
    pub startup_command: Option<String>,
}
impl Default for TerminalPreferences {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            scrollback_lines: 10_000,
            startup_command: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    En,
    ZhCn,
    #[default]
    System,
}
