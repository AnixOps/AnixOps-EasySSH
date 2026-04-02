use gtk4::glib;
use std::collections::HashMap;

/// Server model for GTK bindings
#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "Server")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: AuthType,
    pub group_id: Option<String>,
    pub status: ServerStatus,
    pub identity_file: Option<String>,
}

/// Server group model
#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ServerGroup")]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
    pub server_count: i32,
}

/// Authentication type
#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "AuthType")]
pub enum AuthType {
    Password,
    Key,
    Agent,
}

impl AuthType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthType::Password => "password",
            AuthType::Key => "key",
            AuthType::Agent => "agent",
        }
    }
}

/// Server connection status
#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "ServerStatus")]
pub enum ServerStatus {
    Unknown,
    Connected,
    Disconnected,
    Error,
}

impl ServerStatus {
    pub fn icon_name(&self) -> &'static str {
        match self {
            ServerStatus::Unknown => "network-offline-symbolic",
            ServerStatus::Connected => "network-wired-symbolic",
            ServerStatus::Disconnected => "network-offline-symbolic",
            ServerStatus::Error => "dialog-error-symbolic",
        }
    }
}

/// Application state
pub struct AppState {
    servers: HashMap<String, Server>,
    groups: HashMap<String, ServerGroup>,
    selected_server_id: Option<String>,
    selected_group_id: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        let mut state = Self {
            servers: HashMap::new(),
            groups: HashMap::new(),
            selected_server_id: None,
            selected_group_id: None,
        };
        state.load_sample_data();
        state
    }

    fn load_sample_data(&mut self) {
        // Add default group
        let default_group = ServerGroup {
            id: "default".to_string(),
            name: "Default".to_string(),
            server_count: 2,
        };
        self.groups.insert("default".to_string(), default_group);

        // Add sample servers
        let servers = vec![
            Server {
                id: "1".to_string(),
                name: "Local Server".to_string(),
                host: "localhost".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: AuthType::Key,
                group_id: Some("default".to_string()),
                status: ServerStatus::Disconnected,
                identity_file: Some("~/.ssh/id_rsa".to_string()),
            },
            Server {
                id: "2".to_string(),
                name: "Production".to_string(),
                host: "prod.example.com".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: AuthType::Password,
                group_id: Some("default".to_string()),
                status: ServerStatus::Disconnected,
                identity_file: None,
            },
        ];

        for server in servers {
            self.servers.insert(server.id.clone(), server);
        }
    }

    pub fn get_servers(&self) -> Vec<Server> {
        self.servers.values().cloned().collect()
    }

    pub fn get_servers_by_group(&self, group_id: &str) -> Vec<Server> {
        self.servers
            .values()
            .filter(|s| s.group_id.as_deref() == Some(group_id))
            .cloned()
            .collect()
    }

    pub fn get_server(&self, id: &str) -> Option<Server> {
        self.servers.get(id).cloned()
    }

    pub fn add_server(&mut self, server: Server) {
        self.servers.insert(server.id.clone(), server);
        self.update_group_counts();
    }

    pub fn update_server(&mut self, server: Server) {
        self.servers.insert(server.id.clone(), server);
        self.update_group_counts();
    }

    pub fn delete_server(&mut self, id: &str) {
        self.servers.remove(id);
        self.update_group_counts();
    }

    pub fn get_groups(&self) -> Vec<ServerGroup> {
        self.groups.values().cloned().collect()
    }

    pub fn add_group(&mut self, group: ServerGroup) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn delete_group(&mut self, id: &str) {
        self.groups.remove(id);
        // Move servers to default group
        for server in self.servers.values_mut() {
            if server.group_id.as_deref() == Some(id) {
                server.group_id = Some("default".to_string());
            }
        }
        self.update_group_counts();
    }

    fn update_group_counts(&mut self) {
        for group in self.groups.values_mut() {
            group.server_count = self
                .servers
                .values()
                .filter(|s| s.group_id.as_deref() == Some(&group.id))
                .count() as i32;
        }
    }

    pub fn set_selected_server(&mut self, id: Option<String>) {
        self.selected_server_id = id;
    }

    pub fn get_selected_server(&self) -> Option<Server> {
        self.selected_server_id
            .as_ref()
            .and_then(|id| self.servers.get(id).cloned())
    }

    pub fn set_selected_group(&mut self, id: Option<String>) {
        self.selected_group_id = id;
    }

    pub fn get_selected_group(&self) -> Option<String> {
        self.selected_group_id.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// Conversions from core types
impl From<easyssh_core::ServerRecord> for Server {
    fn from(s: easyssh_core::ServerRecord) -> Self {
        Self {
            id: s.id,
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
            auth_type: match s.auth_type.as_str() {
                "password" => AuthType::Password,
                "key" => AuthType::Key,
                _ => AuthType::Agent,
            },
            group_id: s.group_id,
            status: ServerStatus::Unknown,
            identity_file: s.identity_file,
        }
    }
}

impl From<easyssh_core::GroupRecord> for ServerGroup {
    fn from(g: easyssh_core::GroupRecord) -> Self {
        Self {
            id: g.id,
            name: g.name,
            server_count: 0,
        }
    }
}
