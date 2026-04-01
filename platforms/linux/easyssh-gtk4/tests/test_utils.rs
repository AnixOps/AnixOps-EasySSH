//! Test utilities for GTK4 tests
//!
//! This module provides helper functions and utilities for testing GTK4 widgets
//! in a headless environment.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize GTK for testing (only runs once)
pub fn init_gtk() {
    INIT.call_once(|| {
        gtk4::init().expect("Failed to initialize GTK");
        libadwaita::init();
    });
}

/// Create a headless display for testing
pub fn setup_headless_display() {
    // Set up display for headless testing
    std::env::set_var("DISPLAY", ":99");
}

/// Test helper for creating mock servers
pub fn create_test_server() -> crate::models::Server {
    crate::models::Server {
        id: "test-id".to_string(),
        name: "Test Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "root".to_string(),
        auth_type: crate::models::AuthType::Password,
        group_id: None,
        status: crate::models::ServerStatus::Disconnected,
    }
}

/// Test helper for creating mock server groups
pub fn create_test_group() -> crate::models::ServerGroup {
    crate::models::ServerGroup {
        id: "group-id".to_string(),
        name: "Test Group".to_string(),
        servers: vec!["test-id".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_gtk() {
        init_gtk();
        // Should not panic
    }

    #[test]
    fn test_create_test_server() {
        let server = create_test_server();
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.port, 22);
    }

    #[test]
    fn test_create_test_group() {
        let group = create_test_group();
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.servers.len(), 1);
    }
}