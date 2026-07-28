use crate::domain::{AppConfig, Connection, ConnectionTarget};
use crate::security::serialized_has_sensitive_keys;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("configuration contains disallowed schema fields")]
    SensitiveFields,
}

#[derive(Debug, Default)]
pub struct MigrationReport {
    pub imported_connections: usize,
    pub ignored_sensitive_fields: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn default_path() -> Result<Self, ConfigError> {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("EasySSH");
        Ok(Self::at(dir.join("connections.json")))
    }
    pub fn load(&self) -> Result<AppConfig, ConfigError> {
        if !self.path.exists() {
            return Ok(AppConfig::new());
        }
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&self.path)?)?;
        if serialized_has_sensitive_keys(&value) {
            return Err(ConfigError::SensitiveFields);
        }
        let mut config: AppConfig = serde_json::from_value(value)?;
        config.schema_version = AppConfig::SCHEMA_VERSION;
        Ok(config)
    }
    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        let value = serde_json::to_value(config)?;
        if serialized_has_sensitive_keys(&value) {
            return Err(ConfigError::SensitiveFields);
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&value)?)?;
        fs::rename(temporary, &self.path)?;
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Imports only non-sensitive metadata from a legacy JSON document.
    ///
    /// The caller owns deletion of the old file. This function intentionally
    /// never serializes, logs, or returns legacy authentication values.
    pub fn migrate_legacy_json(
        &self,
        path: &Path,
    ) -> Result<(AppConfig, MigrationReport), ConfigError> {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        let entries = value
            .get("connections")
            .or_else(|| value.get("servers"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut config = AppConfig::new();
        let mut report = MigrationReport::default();
        for entry in entries {
            let Some(object) = entry.as_object() else {
                continue;
            };
            let host = object
                .get("hostname")
                .or_else(|| object.get("host"))
                .and_then(serde_json::Value::as_str);
            let Some(host) = host.filter(|value| !value.trim().is_empty()) else {
                continue;
            };
            let name = object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(host);
            let username = object
                .get("username")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let port = object
                .get("port")
                .and_then(serde_json::Value::as_u64)
                .and_then(|port| u16::try_from(port).ok())
                .unwrap_or(22);
            let mut connection = Connection::alias(name, host);
            connection.target = ConnectionTarget::Endpoint {
                hostname: host.to_owned(),
                username,
                port,
            };
            connection.group_id = object
                .get("group_id")
                .or_else(|| object.get("group"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            connection.tags = object
                .get("tags")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect();
            connection.favorite = object
                .get("favorite")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            config.connections.push(connection);
            report.imported_connections += 1;
        }
        report.ignored_sensitive_fields = serialized_has_sensitive_keys(&value);
        report.warning = report.ignored_sensitive_fields.then(|| "Legacy authentication data was intentionally not migrated. Review and securely delete old credential files.".to_owned());
        Ok((config, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Connection;
    #[test]
    fn saved_schema_has_no_sensitive_property_names() {
        let mut config = AppConfig::new();
        config
            .connections
            .push(Connection::alias("Production", "production"));
        assert!(!serialized_has_sensitive_keys(
            &serde_json::to_value(config).unwrap()
        ));
    }

    #[test]
    fn legacy_migration_keeps_only_connection_metadata() {
        let directory =
            std::env::temp_dir().join(format!("easyssh-migration-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("legacy.json");
        fs::write(&source, r#"{"servers":[{"name":"Production","host":"prod.example","username":"ops","port":2222,"group":"prod","tags":["linux"],"password":"do-not-copy","private_key":"do-not-copy"}]}"#).unwrap();
        let (config, report) = ConfigStore::at(directory.join("connections.json"))
            .migrate_legacy_json(&source)
            .unwrap();
        assert_eq!(report.imported_connections, 1);
        assert!(report.ignored_sensitive_fields);
        let serialized = serde_json::to_value(config).unwrap();
        assert!(!serialized_has_sensitive_keys(&serialized));
        fs::remove_dir_all(directory).unwrap();
    }
}
