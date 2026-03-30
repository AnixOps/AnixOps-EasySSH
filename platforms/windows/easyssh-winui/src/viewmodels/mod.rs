use std::sync::Mutex;
use std::sync::Arc;
use tracing::{info, error};
use easyssh_core::{NewServer, ServerRecord, AppState};

pub struct AppViewModel {
    core_state: Arc<Mutex<AppState>>,
}

impl AppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(AppState::new()));

        // Initialize database
        {
            let state = core_state.lock().unwrap();
            if let Err(e) = easyssh_core::init_database(&state) {
                error!("Failed to initialize database: {}", e);
            } else {
                info!("Database initialized");
            }
        }

        Ok(Self { core_state })
    }

    pub fn get_servers(&self) -> Vec<ServerViewModel> {
        let state = self.core_state.lock().unwrap();
        match easyssh_core::get_servers(&state) {
            Ok(servers) => {
                servers.into_iter().map(|s| ServerViewModel::from(s)).collect()
            }
            Err(e) => {
                error!("Failed to get servers: {}", e);
                vec![]
            }
        }
    }

    pub fn add_server(&self, name: &str, host: &str, port: i64, username: &str, auth_type: &str) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();

        let new_server = NewServer {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            auth_type: auth_type.to_string(),
            identity_file: None,
            group_id: None,
            status: "active".to_string(),
        };

        easyssh_core::add_server(&state, &new_server)?;
        info!("Added server: {}", name);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
}

impl From<ServerRecord> for ServerViewModel {
    fn from(s: ServerRecord) -> Self {
        Self {
            id: s.id,
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
        }
    }
}
