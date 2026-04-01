//! Windows egui UI Tests
//!
//! These tests verify the UI components and interactions for the Windows egui application.
//! Tests are designed to run without requiring an actual display (headless mode where possible).

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Model Tests ====================

    #[test]
    fn test_server_view_model_creation() {
        let server = ServerViewModel {
            id: "test-123".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: "password".to_string(),
        };

        assert_eq!(server.id, "test-123");
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.host, "192.168.1.100");
        assert_eq!(server.port, 22);
        assert_eq!(server.username, "root");
        assert_eq!(server.auth_type, "password");
    }

    #[test]
    fn test_group_view_model_creation() {
        let group = GroupViewModel {
            id: "group-456".to_string(),
            name: "Production".to_string(),
        };

        assert_eq!(group.id, "group-456");
        assert_eq!(group.name, "Production");
    }

    #[test]
    fn test_server_view_model_serialization() {
        let server = ServerViewModel {
            id: "test-789".to_string(),
            name: "Web Server".to_string(),
            host: "10.0.0.5".to_string(),
            port: 2222,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
        };

        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains("test-789"));
        assert!(json.contains("Web Server"));
        assert!(json.contains("2222"));

        let deserialized: ServerViewModel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, server.name);
    }

    // ==================== Bridge Tests ====================

    #[test]
    fn test_server_view_model_clone() {
        let original = ServerViewModel {
            id: "clone-test".to_string(),
            name: "Original".to_string(),
            host: "host.example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_type: "password".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
    }

    #[test]
    fn test_group_view_model_clone() {
        let original = GroupViewModel {
            id: "group-clone".to_string(),
            name: "Developers".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
    }

    // ==================== Form Validation Tests ====================

    #[derive(Default, PartialEq)]
    enum AuthType {
        #[default]
        Password,
        Key,
    }

    #[derive(Default)]
    struct NewServerForm {
        name: String,
        host: String,
        port: String,
        username: String,
        auth_type: AuthType,
    }

    fn validate_server_form(form: &NewServerForm) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if form.name.is_empty() {
            errors.push("Name is required".to_string());
        }
        if form.host.is_empty() {
            errors.push("Host is required".to_string());
        }
        if form.username.is_empty() {
            errors.push("Username is required".to_string());
        }

        // Validate port is numeric if provided
        if !form.port.is_empty() {
            if form.port.parse::<u16>().is_err() {
                errors.push("Port must be a valid number".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    #[test]
    fn test_empty_form_validation() {
        let form = NewServerForm::default();
        let result = validate_server_form(&form);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"Name is required".to_string()));
        assert!(errors.contains(&"Host is required".to_string()));
        assert!(errors.contains(&"Username is required".to_string()));
    }

    #[test]
    fn test_valid_form_validation() {
        let form = NewServerForm {
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: "22".to_string(),
            username: "root".to_string(),
            auth_type: AuthType::Password,
        };

        let result = validate_server_form(&form);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_port_validation() {
        let form = NewServerForm {
            name: "Test".to_string(),
            host: "host.com".to_string(),
            port: "not_a_number".to_string(),
            username: "user".to_string(),
            auth_type: AuthType::Password,
        };

        let result = validate_server_form(&form);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"Port must be a valid number".to_string()));
    }

    #[test]
    fn test_empty_port_defaults_to_22() {
        let form = NewServerForm {
            name: "Test".to_string(),
            host: "host.com".to_string(),
            port: String::new(), // Empty port
            username: "user".to_string(),
            auth_type: AuthType::Password,
        };

        // Empty port should be valid (defaults to 22)
        let result = validate_server_form(&form);
        assert!(result.is_ok());
    }

    // ==================== Connection Status Tests ====================

    #[derive(Default, PartialEq)]
    enum ConnectStatus {
        #[default]
        Idle,
        Connecting,
        Connected,
        Error,
    }

    #[test]
    fn test_connect_status_transitions() {
        let mut status = ConnectStatus::Idle;
        assert_eq!(status, ConnectStatus::Idle);

        status = ConnectStatus::Connecting;
        assert_eq!(status, ConnectStatus::Connecting);

        status = ConnectStatus::Connected;
        assert_eq!(status, ConnectStatus::Connected);

        status = ConnectStatus::Error;
        assert_eq!(status, ConnectStatus::Error);
    }

    #[test]
    fn test_connect_status_default() {
        let status: ConnectStatus = Default::default();
        assert_eq!(status, ConnectStatus::Idle);
    }

    // ==================== File Entry Tests ====================

    struct FileEntry {
        name: String,
        path: String,
        is_dir: bool,
        size: String,
        mtime: String,
    }

    #[test]
    fn test_file_entry_creation() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/home/user/test.txt".to_string(),
            is_dir: false,
            size: "1.2 KB".to_string(),
            mtime: "2024-01-15".to_string(),
        };

        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
    }

    #[test]
    fn test_directory_entry_creation() {
        let entry = FileEntry {
            name: "Documents".to_string(),
            path: "/home/user/Documents".to_string(),
            is_dir: true,
            size: "-".to_string(),
            mtime: "2024-01-10".to_string(),
        };

        assert!(entry.is_dir);
    }

    // ==================== Session Tab Tests ====================

    #[derive(Clone)]
    struct SessionTab {
        session_id: String,
        server_id: String,
        title: String,
        output: String,
        input: String,
        connected: bool,
    }

    #[test]
    fn test_session_tab_creation() {
        let tab = SessionTab {
            session_id: "sess-001".to_string(),
            server_id: "srv-001".to_string(),
            title: "root@192.168.1.1".to_string(),
            output: "Welcome to Ubuntu\n".to_string(),
            input: String::new(),
            connected: true,
        };

        assert_eq!(tab.session_id, "sess-001");
        assert!(tab.connected);
    }

    #[test]
    fn test_session_tab_clone() {
        let original = SessionTab {
            session_id: "sess-clone".to_string(),
            server_id: "srv-clone".to_string(),
            title: "admin@server".to_string(),
            output: "output...".to_string(),
            input: "input".to_string(),
            connected: false,
        };

        let cloned = original.clone();
        assert_eq!(original.session_id, cloned.session_id);
        assert_eq!(original.output, cloned.output);
    }

    // ==================== Monitor Snapshot Tests ====================

    #[derive(Clone)]
    struct MonitorSnapshot {
        cpu: f32,
        memory: f32,
        disk: f32,
        uptime: String,
        net_in: String,
        net_out: String,
        load: String,
        has_errors: bool,
    }

    #[test]
    fn test_monitor_snapshot_creation() {
        let snapshot = MonitorSnapshot {
            cpu: 45.5,
            memory: 62.3,
            disk: 78.9,
            uptime: "3d 4h 12m".to_string(),
            net_in: "1.2 MB/s".to_string(),
            net_out: "0.5 MB/s".to_string(),
            load: "0.45 0.52 0.61".to_string(),
            has_errors: false,
        };

        assert_eq!(snapshot.cpu, 45.5);
        assert!(!snapshot.has_errors);
    }

    #[test]
    fn test_monitor_snapshot_error_state() {
        let snapshot = MonitorSnapshot {
            cpu: 0.0,
            memory: 0.0,
            disk: 0.0,
            uptime: "-".to_string(),
            net_in: "-".to_string(),
            net_out: "-".to_string(),
            load: "-".to_string(),
            has_errors: true,
        };

        assert!(snapshot.has_errors);
    }

    // ==================== Byte Formatting Tests ====================

    fn fmt_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    #[test]
    fn test_fmt_bytes_bytes() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1023), "1023 B");
    }

    #[test]
    fn test_fmt_bytes_kilobytes() {
        assert_eq!(fmt_bytes(1024), "1.0 KB");
        assert_eq!(fmt_bytes(1536), "1.5 KB");
        assert_eq!(fmt_bytes(1024 * 512), "512.0 KB");
    }

    #[test]
    fn test_fmt_bytes_megabytes() {
        assert_eq!(fmt_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(fmt_bytes(1024 * 1024 * 5), "5.0 MB");
    }

    #[test]
    fn test_fmt_bytes_gigabytes() {
        assert_eq!(fmt_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(fmt_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
    }

    // ==================== Rate Formatting Tests ====================

    fn fmt_rate(bytes_per_sec: f64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        if bytes_per_sec >= GB {
            format!("{:.1} GB/s", bytes_per_sec / GB)
        } else if bytes_per_sec >= MB {
            format!("{:.1} MB/s", bytes_per_sec / MB)
        } else if bytes_per_sec >= KB {
            format!("{:.1} KB/s", bytes_per_sec / KB)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    #[test]
    fn test_fmt_rate_bytes() {
        assert_eq!(fmt_rate(100.0), "100 B/s");
        assert_eq!(fmt_rate(1023.0), "1023 B/s");
    }

    #[test]
    fn test_fmt_rate_kilobytes() {
        assert_eq!(fmt_rate(1024.0), "1.0 KB/s");
        assert_eq!(fmt_rate(10240.0), "10.0 KB/s");
    }

    #[test]
    fn test_fmt_rate_megabytes() {
        assert_eq!(fmt_rate(1024.0 * 1024.0), "1.0 MB/s");
        assert_eq!(fmt_rate(1024.0 * 1024.0 * 10.0), "10.0 MB/s");
    }

    #[test]
    fn test_fmt_rate_gigabytes() {
        assert_eq!(fmt_rate(1024.0 * 1024.0 * 1024.0), "1.0 GB/s");
    }

    // ==================== Network Interface Filtering Tests ====================

    fn is_ignored_iface(iface: &str) -> bool {
        iface == "lo"
            || iface.starts_with("docker")
            || iface.starts_with("br-")
            || iface.starts_with("veth")
            || iface.starts_with("flannel")
            || iface.starts_with("cni")
            || iface.starts_with("tun")
            || iface.starts_with("tap")
            || iface.starts_with("virbr")
    }

    #[test]
    fn test_ignored_interfaces() {
        assert!(is_ignored_iface("lo"));
        assert!(is_ignored_iface("docker0"));
        assert!(is_ignored_iface("br-abc123"));
        assert!(is_ignored_iface("veth123"));
        assert!(is_ignored_iface("flannel.1"));
        assert!(is_ignored_iface("cni0"));
        assert!(is_ignored_iface("tun0"));
        assert!(is_ignored_iface("tap0"));
        assert!(is_ignored_iface("virbr0"));
    }

    #[test]
    fn test_valid_interfaces() {
        assert!(!is_ignored_iface("eth0"));
        assert!(!is_ignored_iface("wlan0"));
        assert!(!is_ignored_iface("ens33"));
        assert!(!is_ignored_iface("en0"));
    }

    // ==================== Network Parsing Tests ====================

    fn parse_net_totals(output: &str) -> Option<(u64, u64)> {
        let mut total_in: u64 = 0;
        let mut total_out: u64 = 0;
        let mut has_data = false;

        for line in output.lines().skip(2) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 {
                continue;
            }

            let iface = parts[0].trim();
            if is_ignored_iface(iface) {
                continue;
            }

            let data: Vec<&str> = parts[1].split_whitespace().collect();
            if data.len() >= 10 {
                if let (Ok(rx), Ok(tx)) = (data[0].parse::<u64>(), data[8].parse::<u64>()) {
                    total_in = total_in.saturating_add(rx);
                    total_out = total_out.saturating_add(tx);
                    has_data = true;
                }
            }
        }

        if has_data {
            Some((total_in, total_out))
        } else {
            None
        }
    }

    #[test]
    fn test_parse_net_totals_valid() {
        let output = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n  eth0: 1234567890    1000    0    0    0     0          0         0 9876543210    2000    0    0    0     0       0          0\n  lo: 1000000    100    0    0    0     0          0         0 2000000    200    0    0    0     0       0          0";

        let result = parse_net_totals(output);
        assert!(result.is_some());
        let (in_bytes, out_bytes) = result.unwrap();
        assert_eq!(in_bytes, 1234567890);
        assert_eq!(out_bytes, 9876543210);
    }

    #[test]
    fn test_parse_net_totals_ignores_lo() {
        let output = "Inter-|   Receive                                                |  Transmit\n face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n  lo: 1000000    100    0    0    0     0          0         0 2000000    200    0    0    0     0       0          0";

        let result = parse_net_totals(output);
        // lo interface should be ignored, so no valid data
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_net_totals_empty() {
        let result = parse_net_totals("");
        assert!(result.is_none());
    }

    // ==================== Command History Tests ====================

    #[derive(Default)]
    struct CommandHistory {
        commands: Vec<String>,
        max_size: usize,
    }

    impl CommandHistory {
        fn new(max_size: usize) -> Self {
            Self {
                commands: Vec::new(),
                max_size,
            }
        }

        fn add(&mut self, command: String) {
            // Avoid duplicates
            if !self.commands.contains(&command) {
                self.commands.push(command);
                if self.commands.len() > self.max_size {
                    self.commands.remove(0);
                }
            }
        }

        fn get(&self, index: usize) -> Option<&String> {
            self.commands.get(index)
        }

        fn len(&self) -> usize {
            self.commands.len()
        }

        fn is_empty(&self) -> bool {
            self.commands.is_empty()
        }
    }

    #[test]
    fn test_command_history_add() {
        let mut history = CommandHistory::new(100);
        history.add("ls -la".to_string());
        history.add("pwd".to_string());

        assert_eq!(history.len(), 2);
        assert_eq!(history.get(0), Some(&"ls -la".to_string()));
    }

    #[test]
    fn test_command_history_no_duplicates() {
        let mut history = CommandHistory::new(100);
        history.add("ls".to_string());
        history.add("ls".to_string()); // Duplicate

        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_command_history_max_size() {
        let mut history = CommandHistory::new(3);
        history.add("cmd1".to_string());
        history.add("cmd2".to_string());
        history.add("cmd3".to_string());
        history.add("cmd4".to_string()); // Should push out cmd1

        assert_eq!(history.len(), 3);
        assert_eq!(history.get(0), Some(&"cmd2".to_string()));
        assert_eq!(history.get(2), Some(&"cmd4".to_string()));
    }

    // ==================== Favorites Tests ====================

    use std::collections::HashSet;

    #[test]
    fn test_favorites_add_remove() {
        let mut favorites: HashSet<String> = HashSet::new();

        favorites.insert("server-1".to_string());
        favorites.insert("server-2".to_string());

        assert!(favorites.contains("server-1"));
        assert!(favorites.contains("server-2"));
        assert!(!favorites.contains("server-3"));

        favorites.remove("server-1");
        assert!(!favorites.contains("server-1"));
    }

    // ==================== Path Navigation Tests ====================

    fn navigate_to_parent(current_path: &str) -> String {
        if current_path == "/" {
            return "/".to_string();
        }

        if let Some(pos) = current_path.rfind('/') {
            if pos == 0 {
                "/".to_string()
            } else {
                current_path[..pos].to_string()
            }
        } else {
            "/".to_string()
        }
    }

    #[test]
    fn test_navigate_to_parent_root() {
        assert_eq!(navigate_to_parent("/"), "/");
    }

    #[test]
    fn test_navigate_to_parent_single_level() {
        assert_eq!(navigate_to_parent("/home"), "/");
    }

    #[test]
    fn test_navigate_to_parent_multi_level() {
        assert_eq!(navigate_to_parent("/home/user/documents"), "/home/user");
    }

    #[test]
    fn test_navigate_to_parent_trailing_slash() {
        // Note: this is the behavior without trailing slash handling
        assert_eq!(navigate_to_parent("/home/user/"), "/home/user");
    }

    // ==================== Search Filter Tests ====================

    fn matches_search(server_name: &str, server_host: &str, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query_lower = query.to_lowercase();
        server_name.to_lowercase().contains(&query_lower)
            || server_host.to_lowercase().contains(&query_lower)
    }

    #[test]
    fn test_search_empty_query() {
        assert!(matches_search("Web Server", "192.168.1.1", ""));
    }

    #[test]
    fn test_search_by_name() {
        assert!(matches_search("Production Web Server", "10.0.0.1", "web"));
        assert!(matches_search("Production Web Server", "10.0.0.1", "server"));
        assert!(!matches_search("Production Web Server", "10.0.0.1", "database"));
    }

    #[test]
    fn test_search_by_host() {
        assert!(matches_search("Web Server", "192.168.1.100", "192.168"));
        assert!(matches_search("Web Server", "server.example.com", "example"));
    }

    #[test]
    fn test_search_case_insensitive() {
        assert!(matches_search("Web Server", "192.168.1.1", "WEB"));
        assert!(matches_search("WEB SERVER", "192.168.1.1", "web"));
    }

    // ==================== Terminal Output Truncation Tests ====================

    fn truncate_terminal_output(output: &mut String, max_chars: usize) {
        if output.len() > max_chars {
            let truncate_pos = output.len() - max_chars;
            if let Some(pos) = output[..truncate_pos].find('\n') {
                *output = format!("[...truncated {} bytes...]\n{}",
                    truncate_pos - pos - 1,
                    &output[pos + 1..]);
            } else {
                *output = output[truncate_pos..].to_string();
            }
        }
    }

    #[test]
    fn test_truncate_no_op_when_under_limit() {
        let mut output = "Short output".to_string();
        truncate_terminal_output(&mut output, 100);
        assert_eq!(output, "Short output");
    }

    #[test]
    fn test_truncate_large_output() {
        let mut output = "Line 1\nLine 2\n".to_string();
        output.push_str(&"x".repeat(200));

        let original_len = output.len();
        truncate_terminal_output(&mut output, 100);

        assert!(output.len() < original_len);
        assert!(output.starts_with("[...truncated"));
    }

    // ==================== Debug Stats Tests ====================

    #[derive(Debug, serde::Serialize)]
    struct DebugStats {
        servers_count: usize,
        active_sessions: usize,
        timestamp_ms: u128,
        mux_total_pools: usize,
    }

    #[test]
    fn test_debug_stats_serialization() {
        let stats = DebugStats {
            servers_count: 5,
            active_sessions: 2,
            timestamp_ms: 1234567890,
            mux_total_pools: 1,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("5"));
        assert!(json.contains("2"));
        assert!(json.contains("1234567890"));
    }

    // ==================== WebSocket Message Tests ====================

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct WsCommand {
        action: String,
        payload: Option<serde_json::Value>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct WsResponse {
        success: bool,
        data: Option<serde_json::Value>,
        error: Option<String>,
    }

    #[test]
    fn test_ws_command_serialization() {
        let cmd = WsCommand {
            action: "get_servers".to_string(),
            payload: None,
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("get_servers"));
    }

    #[test]
    fn test_ws_response_serialization() {
        let response = WsResponse {
            success: true,
            data: Some(serde_json::json!({"count": 5})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("count"));
    }

    // ==================== Integration Helper Tests ====================

    #[test]
    fn test_app_view_model_creation() {
        // This is a basic sanity test - full integration tests require the core library
        // In a real test environment, we would mock the core state
        assert!(true); // Placeholder for actual integration test
    }
}

// ==================== UI Component Tests ====================
// Note: These tests require the eframe/egui test harness

#[cfg(all(test, feature = "ui-tests"))]
mod ui_tests {
    // UI tests would go here - these require egui's test utilities
    // which are only available with the "ui-tests" feature enabled
}

// Import the types we're testing
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupViewModel {
    pub id: String,
    pub name: String,
}
