use std::collections::HashMap;

use crate::db::{
    Database, GroupRecord, NewGroup, NewIdentity,
    ServerRecord,
};
use crate::error::LiteError;
use crate::crypto::CryptoState;
use serde::{Deserialize, Serialize};

/// Export format types
#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat {
    Json,
    JsonEncrypted,
    Csv,
    SshConfig,
}

/// Import format types
#[derive(Clone, Debug, PartialEq)]
pub enum ImportFormat {
    Json,
    JsonEncrypted,
    Csv,
    SshConfig,
    AutoDetect,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "JSON"),
            ExportFormat::JsonEncrypted => write!(f, "Encrypted JSON"),
            ExportFormat::Csv => write!(f, "CSV"),
            ExportFormat::SshConfig => write!(f, "SSH Config"),
        }
    }
}

/// Import conflict resolution strategy
#[derive(Clone, Debug, PartialEq)]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    Merge,
}

/// Import result summary
#[derive(Clone, Debug, Default)]
pub struct ImportResult {
    pub servers_imported: usize,
    pub servers_skipped: usize,
    pub servers_merged: usize,
    pub groups_imported: usize,
    pub groups_skipped: usize,
    pub identities_imported: usize,
    pub snippets_imported: usize,
    pub tags_imported: usize,
    pub errors: Vec<String>,
}

impl ImportResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn total_imported(&self) -> usize {
        self.servers_imported + self.groups_imported + self.identities_imported +
        self.snippets_imported + self.tags_imported
    }
}

/// Complete configuration export data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigExport {
    pub version: String,
    pub exported_at: String,
    pub app_version: String,
    pub servers: Vec<ServerExport>,
    pub groups: Vec<GroupExport>,
    pub hosts: Vec<HostExport>,
    pub identities: Vec<IdentityExport>,
    pub snippets: Vec<SnippetExport>,
    pub tags: Vec<TagExport>,
    pub settings: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerExport {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupExport {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostExport {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub identity_name: Option<String>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub environment: Option<String>,
    pub region: Option<String>,
    pub purpose: Option<String>,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IdentityExport {
    pub id: String,
    pub name: String,
    pub private_key_path: Option<String>,
    pub auth_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnippetExport {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub scope: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagExport {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

/// CSV record for server import/export
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerCsvRecord {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: i64,
    pub username: String,
    #[serde(default = "default_auth")]
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group: Option<String>,
    pub tags: Option<String>,
}

fn default_port() -> i64 { 22 }
fn default_auth() -> String { "password".to_string() }

/// Configuration import/export manager
pub struct ConfigManager;

impl ConfigManager {
    /// Export all configuration to JSON
    pub fn export_json(db: &Database, include_secrets: bool) -> Result<String, LiteError> {
        let export = Self::build_export_data(db, include_secrets)?;
        serde_json::to_string_pretty(&export)
            .map_err(|e| LiteError::Config(format!("JSON serialization failed: {}", e)))
    }

    /// Export configuration encrypted with master password
    pub fn export_json_encrypted(
        db: &Database,
        master_password: &str,
        include_secrets: bool,
    ) -> Result<String, LiteError> {
        let export = Self::build_export_data(db, include_secrets)?;
        let json = serde_json::to_string(&export)
            .map_err(|e| LiteError::Config(format!("JSON serialization failed: {}", e)))?;

        // Initialize crypto state with master password
        let mut crypto = CryptoState::new();
        crypto.initialize(master_password)?;

        // Encrypt the JSON data
        let encrypted = crypto.encrypt(json.as_bytes())?;

        // Create encrypted container
        let container = EncryptedExport {
            version: "1.0".to_string(),
            salt: hex_encode(&crypto.get_salt().unwrap_or_default()),
            data: base64_encode(&encrypted),
        };

        serde_json::to_string_pretty(&container)
            .map_err(|e| LiteError::Config(format!("JSON serialization failed: {}", e)))
    }

    /// Export servers to CSV format
    pub fn export_csv(db: &Database) -> Result<String, LiteError> {
        let servers = db.get_servers()?;
        let groups: HashMap<String, String> = db.get_groups()?
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect();

        let mut csv_writer = csv::Writer::from_writer(vec![]);

        for server in servers {
            let record = ServerCsvRecord {
                name: server.name,
                host: server.host,
                port: server.port,
                username: server.username,
                auth_type: server.auth_type,
                identity_file: server.identity_file,
                group: server.group_id.as_ref().and_then(|id| groups.get(id).cloned()),
                tags: None,
            };
            csv_writer.serialize(record)
                .map_err(|e| LiteError::Config(format!("CSV serialization failed: {}", e)))?;
        }

        String::from_utf8(csv_writer.into_inner()
            .map_err(|e| LiteError::Config(format!("CSV write failed: {}", e)))?)
            .map_err(|e| LiteError::Config(format!("Invalid UTF-8 in CSV: {}", e)))
    }

    /// Export to SSH config format (~/.ssh/config style)
    pub fn export_ssh_config(db: &Database) -> Result<String, LiteError> {
        let servers = db.get_servers()?;
        let groups: HashMap<String, String> = db.get_groups()?
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect();

        let mut config = String::new();
        config.push_str("# EasySSH Configuration Export\n");
        config.push_str(&format!("# Generated at: {}\n", chrono_now()));
        config.push_str("#\n\n");

        // Group servers by group
        let mut grouped_servers: HashMap<Option<String>, Vec<&ServerRecord>> = HashMap::new();
        for server in &servers {
            grouped_servers.entry(server.group_id.clone()).or_default().push(server);
        }

        // Export grouped servers first
        for (group_id, servers) in grouped_servers.iter().filter(|(k, _)| k.is_some()) {
            let group_name = groups.get(group_id.as_ref().unwrap());
            if let Some(name) = group_name {
                config.push_str(&format!("# Group: {}\n", name));
            }

            for server in servers {
                config.push_str(&Self::server_to_ssh_config(server));
                config.push('\n');
            }
        }

        // Export ungrouped servers
        if let Some(ungrouped) = grouped_servers.get(&None) {
            if !ungrouped.is_empty() {
                config.push_str("# Ungrouped Servers\n");
                for server in ungrouped {
                    config.push_str(&Self::server_to_ssh_config(server));
                    config.push('\n');
                }
            }
        }

        Ok(config)
    }

    /// Import configuration from JSON
    pub fn import_json(
        db: &Database,
        json: &str,
        conflict_resolution: ConflictResolution,
    ) -> Result<ImportResult, LiteError> {
        let export: ConfigExport = serde_json::from_str(json)
            .map_err(|e| LiteError::Config(format!("Invalid JSON format: {}", e)))?;

        Self::import_from_export(db, export, conflict_resolution)
    }

    /// Import from encrypted JSON
    pub fn import_json_encrypted(
        db: &Database,
        encrypted_json: &str,
        master_password: &str,
        conflict_resolution: ConflictResolution,
    ) -> Result<ImportResult, LiteError> {
        let container: EncryptedExport = serde_json::from_str(encrypted_json)
            .map_err(|e| LiteError::Config(format!("Invalid encrypted export format: {}", e)))?;

        // Decode salt
        let salt = hex_decode(&container.salt)
            .map_err(|e| LiteError::Config(format!("Invalid salt: {}", e)))?;
        let salt_array: [u8; 32] = salt.try_into()
            .map_err(|_| LiteError::Config("Invalid salt length".to_string()))?;

        // Initialize crypto and unlock
        let mut crypto = CryptoState::new();
        crypto.set_salt(salt_array);

        if !crypto.unlock(master_password)? {
            return Err(LiteError::InvalidMasterPassword);
        }

        // Decrypt data
        let encrypted_data = base64_decode(&container.data)
            .map_err(|e| LiteError::Config(format!("Invalid base64 data: {}", e)))?;
        let decrypted = crypto.decrypt(&encrypted_data)?;
        let json = String::from_utf8(decrypted)
            .map_err(|e| LiteError::Config(format!("Invalid UTF-8: {}", e)))?;

        Self::import_json(db, &json, conflict_resolution)
    }

    /// Import servers from CSV
    pub fn import_csv(
        db: &Database,
        csv_content: &str,
        conflict_resolution: ConflictResolution,
    ) -> Result<ImportResult, LiteError> {
        let mut result = ImportResult::default();
        let existing_servers = db.get_servers()?;
        let existing_groups = db.get_groups()?;

        // Build lookup maps
        let existing_by_host_user: HashMap<(String, String), &ServerRecord> = existing_servers
            .iter()
            .map(|s| ((s.host.clone(), s.username.clone()), s))
            .collect();

        let group_name_to_id: HashMap<String, String> = existing_groups
            .iter()
            .map(|g| (g.name.clone(), g.id.clone()))
            .collect();

        let mut csv_reader = csv::Reader::from_reader(csv_content.as_bytes());

        for record in csv_reader.deserialize::<ServerCsvRecord>() {
            let record = match record {
                Ok(r) => r,
                Err(e) => {
                    result.errors.push(format!("CSV parse error: {}", e));
                    continue;
                }
            };

            let host_user_key = (record.host.clone(), record.username.clone());

            // Check for conflicts
            if let Some(existing) = existing_by_host_user.get(&host_user_key) {
                match conflict_resolution {
                    ConflictResolution::Skip => {
                        result.servers_skipped += 1;
                        continue;
                    }
                    ConflictResolution::Overwrite => {
                        // Update existing server
                        let update = crate::db::UpdateServer {
                            id: existing.id.clone(),
                            name: record.name.clone(),
                            host: record.host.clone(),
                            port: record.port,
                            username: record.username.clone(),
                            auth_type: record.auth_type.clone(),
                            identity_file: record.identity_file.clone(),
                            group_id: record.group.as_ref()
                                .and_then(|g| group_name_to_id.get(g).cloned()),
                            status: "active".to_string(),
                        };
                        if let Err(e) = db.update_server(&update) {
                            result.errors.push(format!("Failed to update server: {}", e));
                        } else {
                            result.servers_imported += 1;
                        }
                        continue;
                    }
                    ConflictResolution::Merge => {
                        // Keep existing, skip
                        result.servers_skipped += 1;
                        continue;
                    }
                }
            }

            // Create new server
            let group_id = if let Some(ref group_name) = record.group {
                if let Some(id) = group_name_to_id.get(group_name) {
                    Some(id.clone())
                } else {
                    // Create new group
                    let new_group = NewGroup {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: group_name.clone(),
                    };
                    if let Err(e) = db.add_group(&new_group) {
                        result.errors.push(format!("Failed to create group: {}", e));
                        None
                    } else {
                        result.groups_imported += 1;
                        Some(new_group.id)
                    }
                }
            } else {
                None
            };

            let new_server = crate::db::NewServer {
                id: uuid::Uuid::new_v4().to_string(),
                name: record.name,
                host: record.host,
                port: record.port,
                username: record.username,
                auth_type: record.auth_type,
                identity_file: record.identity_file,
                group_id,
                status: "active".to_string(),
            };

            if let Err(e) = db.add_server(&new_server) {
                result.errors.push(format!("Failed to add server: {}", e));
            } else {
                result.servers_imported += 1;
            }
        }

        Ok(result)
    }

    /// Parse SSH config file and import servers
    pub fn import_ssh_config(
        db: &Database,
        config_content: &str,
        conflict_resolution: ConflictResolution,
    ) -> Result<ImportResult, LiteError> {
        let mut result = ImportResult::default();
        let existing_servers = db.get_servers()?;
        let existing_groups = db.get_groups()?;

        let existing_by_host_user: HashMap<(String, String), &ServerRecord> = existing_servers
            .iter()
            .map(|s| ((s.host.clone(), s.username.clone()), s))
            .collect();

        let group_name_to_id: HashMap<String, String> = existing_groups
            .iter()
            .map(|g| (g.name.clone(), g.id.clone()))
            .collect();

        let hosts = Self::parse_ssh_config(config_content)?;

        for host in hosts {
            let host_user_key = (host.host.clone(), host.username.clone());

            // Check for conflicts
            if let Some(existing) = existing_by_host_user.get(&host_user_key) {
                match conflict_resolution {
                    ConflictResolution::Skip => {
                        result.servers_skipped += 1;
                        continue;
                    }
                    ConflictResolution::Overwrite => {
                        let update = crate::db::UpdateServer {
                            id: existing.id.clone(),
                            name: host.name.clone(),
                            host: host.host.clone(),
                            port: host.port,
                            username: host.username.clone(),
                            auth_type: host.auth_type.clone(),
                            identity_file: host.identity_file.clone(),
                            group_id: host.group_name.as_ref()
                                .and_then(|g| group_name_to_id.get(g).cloned()),
                            status: "active".to_string(),
                        };
                        if let Err(e) = db.update_server(&update) {
                            result.errors.push(format!("Failed to update server: {}", e));
                        } else {
                            result.servers_imported += 1;
                        }
                        continue;
                    }
                    ConflictResolution::Merge => {
                        result.servers_skipped += 1;
                        continue;
                    }
                }
            }

            // Get or create group
            let group_id = if let Some(ref group_name) = host.group_name {
                if let Some(id) = group_name_to_id.get(group_name) {
                    Some(id.clone())
                } else {
                    let new_group = NewGroup {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: group_name.clone(),
                    };
                    if let Err(e) = db.add_group(&new_group) {
                        result.errors.push(format!("Failed to create group: {}", e));
                        None
                    } else {
                        result.groups_imported += 1;
                        Some(new_group.id)
                    }
                }
            } else {
                None
            };

            let new_server = crate::db::NewServer {
                id: uuid::Uuid::new_v4().to_string(),
                name: host.name,
                host: host.host,
                port: host.port,
                username: host.username,
                auth_type: host.auth_type,
                identity_file: host.identity_file,
                group_id,
                status: "active".to_string(),
            };

            if let Err(e) = db.add_server(&new_server) {
                result.errors.push(format!("Failed to add server: {}", e));
            } else {
                result.servers_imported += 1;
            }
        }

        Ok(result)
    }

    // Internal helper methods
    fn build_export_data(db: &Database, _include_secrets: bool) -> Result<ConfigExport, LiteError> {
        let servers = db.get_servers()?;
        let groups = db.get_groups()?;
        let hosts = db.get_hosts()?;
        let identities = db.get_identities()?;
        let snippets = db.get_snippets()?;
        let tags = db.get_tags()?;

        // Build lookup maps
        let group_map: HashMap<String, String> = groups
            .iter()
            .map(|g| (g.id.clone(), g.name.clone()))
            .collect();

        let identity_map: HashMap<String, String> = identities
            .iter()
            .map(|i| (i.id.clone(), i.name.clone()))
            .collect();

        // Export servers
        let servers_export: Vec<ServerExport> = servers
            .into_iter()
            .map(|s| ServerExport {
                id: s.id.clone(),
                name: s.name,
                host: s.host,
                port: s.port,
                username: s.username,
                auth_type: s.auth_type,
                identity_file: s.identity_file,
                group_id: s.group_id.clone(),
                group_name: s.group_id.as_ref().and_then(|id| group_map.get(id)).cloned(),
                status: s.status,
                tags: vec![], // TODO: Get actual tags
            })
            .collect();

        // Export groups
        let groups_export: Vec<GroupExport> = groups
            .into_iter()
            .map(|g| GroupExport {
                id: g.id,
                name: g.name,
                parent_id: None, // TODO: Support nested groups
            })
            .collect();

        // Export hosts
        let hosts_export: Vec<HostExport> = hosts
            .into_iter()
            .map(|h| HostExport {
                id: h.id.clone(),
                name: h.name,
                host: h.host,
                port: h.port,
                username: h.username,
                auth_type: h.auth_type,
                identity_file: h.identity_file,
                identity_name: h.identity_id.as_ref().and_then(|id| identity_map.get(id)).cloned(),
                group_id: h.group_id.clone(),
                group_name: h.group_id.as_ref().and_then(|id| group_map.get(id)).cloned(),
                notes: h.notes,
                color: h.color,
                environment: h.environment,
                region: h.region,
                purpose: h.purpose,
                status: h.status,
                tags: vec![], // TODO: Get actual tags
            })
            .collect();

        // Export identities
        let identities_export: Vec<IdentityExport> = identities
            .into_iter()
            .map(|i| IdentityExport {
                id: i.id,
                name: i.name,
                private_key_path: i.private_key_path,
                auth_type: i.auth_type,
            })
            .collect();

        // Export snippets
        let snippets_export: Vec<SnippetExport> = snippets
            .into_iter()
            .map(|s| SnippetExport {
                id: s.id,
                name: s.name,
                command: s.command,
                description: s.description,
                folder_id: s.folder_id,
                scope: s.scope,
                tags: vec![], // TODO: Get actual tags
            })
            .collect();

        // Export tags
        let tags_export: Vec<TagExport> = tags
            .into_iter()
            .map(|t| TagExport {
                id: t.id,
                name: t.name,
                color: t.color,
                description: t.description,
            })
            .collect();

        Ok(ConfigExport {
            version: "1.0".to_string(),
            exported_at: chrono_now(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            servers: servers_export,
            groups: groups_export,
            hosts: hosts_export,
            identities: identities_export,
            snippets: snippets_export,
            tags: tags_export,
            settings: None, // TODO: Export settings
        })
    }

    fn import_from_export(
        db: &Database,
        export: ConfigExport,
        conflict_resolution: ConflictResolution,
    ) -> Result<ImportResult, LiteError> {
        let mut result = ImportResult::default();
        let existing_servers = db.get_servers()?;
        let existing_groups = db.get_groups()?;

        // Build lookup maps
        let existing_by_id: HashMap<String, &ServerRecord> = existing_servers
            .iter()
            .map(|s| (s.id.clone(), s))
            .collect();

        let existing_by_host_user: HashMap<(String, String), &ServerRecord> = existing_servers
            .iter()
            .map(|s| ((s.host.clone(), s.username.clone()), s))
            .collect();

        let existing_group_by_name: HashMap<String, &GroupRecord> = existing_groups
            .iter()
            .map(|g| (g.name.clone(), g))
            .collect();

        // First, import groups
        let mut group_id_mapping: HashMap<String, String> = HashMap::new(); // old_id -> new_id
        for group in export.groups {
            if let Some(existing) = existing_group_by_name.get(&group.name) {
                group_id_mapping.insert(group.id, existing.id.clone());
                result.groups_skipped += 1;
            } else {
                let new_id = uuid::Uuid::new_v4().to_string();
                let new_group = NewGroup {
                    id: new_id.clone(),
                    name: group.name,
                };
                if let Err(e) = db.add_group(&new_group) {
                    result.errors.push(format!("Failed to add group: {}", e));
                } else {
                    group_id_mapping.insert(group.id, new_id);
                    result.groups_imported += 1;
                }
            }
        }

        // Import servers
        for server in export.servers {
            let host_user_key = (server.host.clone(), server.username.clone());

            // Check for conflicts
            if existing_by_id.contains_key(&server.id) || existing_by_host_user.contains_key(&host_user_key) {
                match conflict_resolution {
                    ConflictResolution::Skip => {
                        result.servers_skipped += 1;
                        continue;
                    }
                    ConflictResolution::Overwrite => {
                        // Find existing to update
                        if let Some(existing) = existing_by_id.get(&server.id)
                            .or_else(|| existing_by_host_user.get(&host_user_key)) {
                            let update = crate::db::UpdateServer {
                                id: existing.id.clone(),
                                name: server.name,
                                host: server.host,
                                port: server.port,
                                username: server.username,
                                auth_type: server.auth_type,
                                identity_file: server.identity_file,
                                group_id: server.group_id.as_ref()
                                    .and_then(|id| group_id_mapping.get(id).cloned()),
                                status: server.status,
                            };
                            if let Err(e) = db.update_server(&update) {
                                result.errors.push(format!("Failed to update server: {}", e));
                            } else {
                                result.servers_imported += 1;
                            }
                        }
                        continue;
                    }
                    ConflictResolution::Merge => {
                        result.servers_skipped += 1;
                        continue;
                    }
                }
            }

            // Create new server with mapped group_id
            let new_server = crate::db::NewServer {
                id: uuid::Uuid::new_v4().to_string(),
                name: server.name,
                host: server.host,
                port: server.port,
                username: server.username,
                auth_type: server.auth_type,
                identity_file: server.identity_file,
                group_id: server.group_id.as_ref()
                    .and_then(|id| group_id_mapping.get(id).cloned()),
                status: server.status,
            };

            if let Err(e) = db.add_server(&new_server) {
                result.errors.push(format!("Failed to add server: {}", e));
            } else {
                result.servers_imported += 1;
            }
        }

        // Import identities
        for identity in export.identities {
            let new_identity = NewIdentity {
                id: uuid::Uuid::new_v4().to_string(),
                name: identity.name,
                private_key_path: identity.private_key_path,
                passphrase_secret_id: None,
                auth_type: identity.auth_type,
            };
            if let Err(e) = db.add_identity(&new_identity) {
                result.errors.push(format!("Failed to add identity: {}", e));
            } else {
                result.identities_imported += 1;
            }
        }

        // Import snippets
        for snippet in export.snippets {
            let new_snippet = crate::db::NewSnippet {
                id: uuid::Uuid::new_v4().to_string(),
                name: snippet.name,
                command: snippet.command,
                description: snippet.description,
                folder_id: snippet.folder_id,
                variables_json: None,
                scope: snippet.scope,
            };
            if let Err(e) = db.add_snippet(&new_snippet) {
                result.errors.push(format!("Failed to add snippet: {}", e));
            } else {
                result.snippets_imported += 1;
            }
        }

        // Import tags
        for tag in export.tags {
            let new_tag = crate::db::NewTag {
                id: uuid::Uuid::new_v4().to_string(),
                name: tag.name,
                color: tag.color,
                description: tag.description,
            };
            if let Err(e) = db.add_tag(&new_tag) {
                result.errors.push(format!("Failed to add tag: {}", e));
            } else {
                result.tags_imported += 1;
            }
        }

        Ok(result)
    }

    fn server_to_ssh_config(server: &ServerRecord) -> String {
        let mut config = String::new();
        config.push_str(&format!("Host {}\n", server.name));
        config.push_str(&format!("    HostName {}\n", server.host));
        config.push_str(&format!("    Port {}\n", server.port));
        config.push_str(&format!("    User {}\n", server.username));

        if let Some(ref identity) = server.identity_file {
            config.push_str(&format!("    IdentityFile {}\n", identity));
        }

        match server.auth_type.as_str() {
            "key" => {
                // SSH key auth - IdentityFile already added above
            }
            "agent" => {
                config.push_str("    ForwardAgent yes\n");
            }
            _ => {
                // Password auth - no specific config needed
            }
        }

        config.push_str("    StrictHostKeyChecking accept-new\n");
        config
    }

    fn parse_ssh_config(content: &str) -> Result<Vec<SshConfigHost>, LiteError> {
        let mut hosts = Vec::new();
        let mut current_host: Option<SshConfigHost> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key-value pairs
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let key = parts[0].to_lowercase();
            let value = parts[1..].join(" ");

            match key.as_str() {
                "host" => {
                    // Save previous host if exists
                    if let Some(host) = current_host.take() {
                        if !host.name.starts_with('*') { // Skip wildcard patterns
                            hosts.push(host);
                        }
                    }

                    // Start new host
                    current_host = Some(SshConfigHost {
                        name: value.clone(),
                        host: value.clone(), // Will be updated if HostName is present
                        port: 22,
                        username: String::from("root"),
                        auth_type: String::from("password"),
                        identity_file: None,
                        group_name: None,
                    });
                }
                "hostname" => {
                    if let Some(ref mut host) = current_host {
                        host.host = value;
                    }
                }
                "port" => {
                    if let Some(ref mut host) = current_host {
                        host.port = value.parse().unwrap_or(22);
                    }
                }
                "user" => {
                    if let Some(ref mut host) = current_host {
                        host.username = value;
                    }
                }
                "identityfile" => {
                    if let Some(ref mut host) = current_host {
                        host.identity_file = Some(value.replace("~", &std::env::var("HOME").unwrap_or_default()));
                        host.auth_type = String::from("key");
                    }
                }
                "forwardagent" => {
                    if let Some(ref mut host) = current_host {
                        if value.to_lowercase() == "yes" {
                            host.auth_type = String::from("agent");
                        }
                    }
                }
                _ => {}
            }
        }

        // Don't forget the last host
        if let Some(host) = current_host {
            if !host.name.starts_with('*') {
                hosts.push(host);
            }
        }

        Ok(hosts)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct EncryptedExport {
    pub version: String,
    pub salt: String,
    pub data: String,
}

#[derive(Clone, Debug)]
struct SshConfigHost {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_name: Option<String>,
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

// Base64 encoding/decoding helpers using the base64 crate
use base64::{Engine as _, engine::general_purpose};

fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, LiteError> {
    general_purpose::STANDARD.decode(s)
        .map_err(|e| LiteError::Config(format!("Base64 decode failed: {}", e)))
}

// Hex encoding/decoding helpers
fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

fn hex_decode(s: &str) -> Result<Vec<u8>, LiteError> {
    let mut result = Vec::new();

    for i in (0..s.len()).step_by(2) {
        if i + 1 >= s.len() {
            return Err(LiteError::Config("Invalid hex string length".to_string()));
        }
        let hex_byte = &s[i..i+2];
        let byte = u8::from_str_radix(hex_byte, 16)
            .map_err(|e| LiteError::Config(format!("Invalid hex: {}", e)))?;
        result.push(byte);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_parsing() {
        let config = r#"
# My servers
Host myserver
    HostName 192.168.1.100
    Port 2222
    User admin
    IdentityFile ~/.ssh/id_rsa

Host production
    HostName prod.example.com
    User root
    ForwardAgent yes
"#;

        let hosts = ConfigManager::parse_ssh_config(config).unwrap();
        assert_eq!(hosts.len(), 2);

        assert_eq!(hosts[0].name, "myserver");
        assert_eq!(hosts[0].host, "192.168.1.100");
        assert_eq!(hosts[0].port, 2222);
        assert_eq!(hosts[0].username, "admin");
        assert!(hosts[0].identity_file.is_some());
        assert_eq!(hosts[0].auth_type, "key");

        assert_eq!(hosts[1].name, "production");
        assert_eq!(hosts[1].auth_type, "agent");
    }

    #[test]
    fn test_server_to_ssh_config() {
        let server = ServerRecord {
            id: "test-id".to_string(),
            name: "TestServer".to_string(),
            host: "192.168.1.100".to_string(),
            port: 2222,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            identity_file: Some("/path/to/key".to_string()),
            group_id: None,
            status: "active".to_string(),
            created_at: "123456".to_string(),
            updated_at: "123456".to_string(),
        };

        let config = ConfigManager::server_to_ssh_config(&server);
        assert!(config.contains("Host TestServer"));
        assert!(config.contains("HostName 192.168.1.100"));
        assert!(config.contains("Port 2222"));
        assert!(config.contains("User admin"));
        assert!(config.contains("IdentityFile /path/to/key"));
    }
}
