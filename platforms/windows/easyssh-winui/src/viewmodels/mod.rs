use std::sync::Mutex;
use std::sync::Arc;
use tracing::{info, error};
use crate::bridge::ServerViewModel as BridgeServer;

pub struct AppViewModel {
    _core_state: Arc<Mutex<easyssh_core::AppState>>,
}

impl AppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(easyssh_core::AppState::new()));

        // Initialize database
        {
            let state = core_state.lock().unwrap();
            if let Err(e) = easyssh_core::init_database(&state) {
                error!("Failed to initialize database: {}", e);
            } else {
                info!("Database initialized");
            }
        }

        Ok(Self {
            _core_state: core_state,
        })
    }

    pub fn get_servers(&self) -> Vec<ServerViewModel> {
        let state = self._core_state.lock().unwrap();
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
}

#[derive(Clone, Debug)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
}

impl From<easyssh_core::ServerRecord> for ServerViewModel {
    fn from(s: easyssh_core::ServerRecord) -> Self {
        Self {
            id: s.id,
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
        }
    }
}
