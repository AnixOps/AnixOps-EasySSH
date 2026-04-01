//! Test utilities for Windows egui tests
//!
//! This module provides helper functions for testing egui UI components.

// Define local types for integration tests (can't use crate:: paths in integration tests)
#[derive(Clone, Debug)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub group_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GroupViewModel {
    pub id: String,
    pub name: String,
}

/// Test helper for creating mock server view models
pub fn create_test_server_viewmodel() -> ServerViewModel {
    ServerViewModel {
        id: "test-id".to_string(),
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "root".to_string(),
        group_id: None,
    }
}

/// Test helper for creating mock group view models
pub fn create_test_group_viewmodel() -> GroupViewModel {
    GroupViewModel {
        id: "group-id".to_string(),
        name: "Test Group".to_string(),
    }
}

/// Test helper for mock WebSocket messages
pub fn create_test_ws_command(action: &str) -> serde_json::Value {
    serde_json::json!({
        "action": action,
        "payload": null
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_server() {
        let server = create_test_server_viewmodel();
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.port, 22);
    }

    #[test]
    fn test_create_test_group() {
        let group = create_test_group_viewmodel();
        assert_eq!(group.name, "Test Group");
    }

    #[test]
    fn test_create_ws_command() {
        let cmd = create_test_ws_command("get_servers");
        assert_eq!(cmd["action"], "get_servers");
    }
}
