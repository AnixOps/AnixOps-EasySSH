//! Tests for russh implementation

#[cfg(feature = "russh-backend")]
mod russh_backend_tests {
    use crate::russh_impl::config::*;
    use crate::russh_impl::error::*;
    use crate::russh_impl::client::*;
    use crate::russh_impl::session::*;
    use crate::russh_impl::channel::*;
    use crate::russh_impl::manager::*;

    #[test]
    fn test_config_creation() {
        let config = RusshConfig::new("192.168.1.1", 22, "root");

        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 22);
        assert_eq!(config.username, "root");
        assert!(config.auth.is_agent());
        assert!(config.is_valid());
    }

    #[test]
    fn test_config_with_password() {
        let config = RusshConfig::with_password("host", 2222, "user", "secret");

        assert!(config.auth.is_password());
        assert!(config.is_valid());
    }

    #[test]
    fn test_config_with_key() {
        let config = RusshConfig::with_key(
            "host",
            22,
            "user",
            std::path::PathBuf::from("/home/user/.ssh/id_rsa"),
            None,
        );

        assert!(config.auth.is_public_key());
    }

    #[test]
    fn test_auth_method_validity() {
        assert!(RusshAuthMethod::password("test").is_valid());
        assert!(!RusshAuthMethod::password("").is_valid());
        assert!(RusshAuthMethod::agent().is_valid());
        assert!(!RusshAuthMethod::None.is_valid());
    }

    #[test]
    fn test_auth_method_clear_sensitive() {
        let mut auth = RusshAuthMethod::password("secret_password");
        auth.clear_sensitive_data();

        if let RusshAuthMethod::Password(p) = &auth {
            assert!(p.is_empty() || p.as_bytes().iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_known_hosts_policy() {
        assert!(RusshKnownHostsPolicy::Strict.requires_verification());
        assert!(!RusshKnownHostsPolicy::Ignore.requires_verification());
        assert!(RusshKnownHostsPolicy::AcceptNew.auto_accept_new());
        assert!(!RusshKnownHostsPolicy::Strict.auto_accept_new());
    }

    #[test]
    fn test_timeout_config() {
        let timeout = RusshTimeout::default();

        assert_eq!(timeout.connect_secs, 10);
        assert_eq!(timeout.auth_secs, 30);
        assert_eq!(timeout.keepalive_secs, 30);
        assert_eq!(timeout.command_secs, 60);

        let aggressive = RusshTimeout::aggressive();
        assert_eq!(aggressive.connect_secs, 5);

        let relaxed = RusshTimeout::relaxed();
        assert_eq!(relaxed.connect_secs, 30);
    }

    #[test]
    fn test_error_retryable() {
        let err = RusshError::ConnectionFailed {
            host: "test".into(),
            port: 22,
            message: "Connection reset by peer".into(),
        };
        assert!(err.is_retryable());

        let err = RusshError::AuthFailed {
            host: "test".into(),
            username: "root".into(),
            reason: "Invalid password".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_user_suggestion() {
        let err = RusshError::AuthFailed {
            host: "test".into(),
            username: "root".into(),
            reason: "Invalid password".into(),
        };
        assert!(err.user_suggestion().is_some());

        let err = RusshError::Timeout { seconds: 30 };
        assert!(err.user_suggestion().is_some());
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(RusshError::Timeout { seconds: 30 }.error_code(), "R1009");
        assert_eq!(
            RusshError::AuthFailed {
                host: "test".into(),
                username: "root".into(),
                reason: "test".into()
            }
            .error_code(),
            "R1002"
        );
    }

    #[test]
    fn test_session_state() {
        assert!(RusshSessionState::Active.is_ready());
        assert!(!RusshSessionState::Idle.is_ready());

        assert!(RusshSessionState::Failed.can_reconnect());
        assert!(!RusshSessionState::Active.can_reconnect());

        assert!(RusshSessionState::Disconnected.is_terminal());
        assert!(!RusshSessionState::Active.is_terminal());
    }

    #[test]
    fn test_exec_result() {
        let result = RusshExecResult {
            exit_code: 0,
            stdout: "output".into(),
            stderr: String::new(),
        };

        assert!(result.success());
        assert_eq!(result.combined_output(), "output");
    }

    #[test]
    fn test_scroll_buffer() {
        let mut buffer = ScrollBuffer::new(100);

        buffer.push("line 1".into());
        buffer.push("line 2".into());
        buffer.push("line 3".into());

        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());

        let results = buffer.search("line");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_scroll_buffer_fifo() {
        let mut buffer = ScrollBuffer::new(3);

        for i in 0..10 {
            buffer.push(format!("line {}", i));
        }

        // Should only keep last 3
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.lines()[0], "line 7");
        assert_eq!(buffer.lines()[2], "line 9");
    }

    #[test]
    fn test_reconnect_config() {
        use crate::russh_impl::RusshReconnectConfig;
        let config = RusshReconnectConfig::default();

        assert_eq!(config.max_retries, 10);
        assert_eq!(config.base_delay, std::time::Duration::from_secs(1));
        assert_eq!(config.max_delay, std::time::Duration::from_secs(60));

        let d0 = config.calculate_delay(0);
        let d1 = config.calculate_delay(1);
        assert!(d1 >= d0);
    }

    #[test]
    fn test_pool_manager() {
        let manager = RusshSessionManager::new();

        assert!(manager.list_sessions().is_empty());
        assert!(!manager.has_session("test"));

        let stats = manager.get_pool_stats();
        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.total_sessions, 0);
    }

    #[test]
    fn test_pool_manager_config() {
        let manager = RusshSessionManager::new()
            .with_pool_config(10, 600, 7200);

        assert_eq!(manager.max_connections(), 10);
        assert_eq!(manager.idle_timeout_secs(), 600);
        assert_eq!(manager.max_age_secs(), 7200);
    }

    #[test]
    fn test_jump_host_config() {
        let jump = JumpHostConfig::new("jumphost", 22, "jumpuser")
            .with_auth(RusshAuthMethod::agent());

        assert!(jump.is_valid());
        assert_eq!(jump.address(), "jumphost:22");
    }
}

mod non_backend_tests {
    #[test]
    fn test_basic_types_exist() {
        // This test ensures the module compiles even without russh-backend feature
        assert!(true);
    }
}