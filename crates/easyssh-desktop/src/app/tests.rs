use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_includes_endpoint_user_and_port() {
        let mut c = Connection::alias("x", "prod");
        c.target = ConnectionTarget::Endpoint {
            hostname: "host".into(),
            username: Some("ops".into()),
            port: 2200,
        };
        assert_eq!(target_text(&c), "ops@host:2200");
    }
    #[test]
    fn relative_time_is_compact() {
        assert_eq!(relative_time(Utc::now()), "now");
    }

    #[test]
    fn quick_connect_parses_user_host_and_port() {
        assert_eq!(
            parse_quick_target("ops@example.com:2200", "", 22),
            (Some("ops".into()), "example.com".into(), 2200)
        );
        assert_eq!(
            parse_quick_target("example.com", "ops", 22),
            (Some("ops".into()), "example.com".into(), 22)
        );
    }

    #[test]
    fn ssh_config_import_is_idempotent_and_uses_alias_targets() {
        let mut connections = Vec::new();
        let aliases = vec!["production".to_owned(), "staging".to_owned()];
        assert_eq!(import_aliases(&mut connections, &aliases), (2, 0));
        assert!(matches!(
            connections[0].target,
            ConnectionTarget::Alias { ref alias } if alias == "production"
        ));
        assert_eq!(import_aliases(&mut connections, &aliases), (0, 2));
    }
}
