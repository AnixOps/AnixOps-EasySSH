use std::sync::Mutex;
use std::sync::Arc;
use tracing::{info, error};
use easyssh_core::{NewServer, ServerRecord, AppState, SshSessionManager};
use tokio::runtime::Runtime;

pub struct AppViewModel {
    core_state: Arc<Mutex<AppState>>,
    ssh_manager: Arc<Mutex<SshSessionManager>>,
    runtime: Arc<Runtime>,
}

impl AppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(AppState::new()));
        let ssh_manager = Arc::new(Mutex::new(SshSessionManager::new()));
        let runtime = Arc::new(Runtime::new()?);

        // Initialize database
        {
            let state = core_state.lock().unwrap();
            if let Err(e) = easyssh_core::init_database(&state) {
                error!("Failed to initialize database: {}", e);
            } else {
                info!("Database initialized");
            }
        }

        Ok(Self { core_state, ssh_manager, runtime })
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

    pub fn connect(&self, session_id: &str, host: &str, port: i64, username: &str, password: Option<&str>) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();

        rt.block_on(async {
            let mut mgr = manager.lock().unwrap();
            mgr.connect(session_id, host, port as u16, username, password).await
                .map_err(|e| anyhow::anyhow!("SSH connection failed: {}", e))
        })
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
