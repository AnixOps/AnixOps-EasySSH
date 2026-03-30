use gtk4::glib;

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
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ServerGroup")]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
    pub servers: Vec<String>, // Server IDs
}

#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "AuthType")]
pub enum AuthType {
    Password,
    Key,
    Agent,
}

#[derive(Clone, Debug, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "ServerStatus")]
pub enum ServerStatus {
    Unknown,
    Connected,
    Disconnected,
    Error,
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
        }
    }
}

impl From<easyssh_core::GroupRecord> for ServerGroup {
    fn from(g: easyssh_core::GroupRecord) -> Self {
        Self {
            id: g.id,
            name: g.name,
            servers: Vec::new(), // TODO: Load from relation
        }
    }
}
