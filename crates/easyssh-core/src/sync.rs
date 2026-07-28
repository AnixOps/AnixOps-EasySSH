//! Explicit, whitelist-only Git metadata synchronization.
//!
//! This module never reads Git credentials, private keys, terminal output, or
//! SSH command lines. It delegates authentication to the user's system Git.
use crate::domain::{AppConfig, CommandSnippet, Connection, ConnectionTarget, Group};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

pub const SYNC_FILE_NAME: &str = ".easyssh-workbench.json";

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("sync repository is not configured")]
    NotConfigured,
    #[error("Git operation failed: {0}")]
    GitFailed(String),
    #[error("sync metadata is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sync metadata I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("remote URL must not embed credentials; use Git Credential Manager or SSH")]
    EmbeddedCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Unconfigured,
    Clean,
    LocalChanges,
    RemoteUpdates,
    Conflict,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SyncFile {
    pub version: u32,
    pub connections: Vec<SyncConnection>,
    pub groups: Vec<SyncGroup>,
    pub snippets: Vec<SyncSnippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SyncConnection {
    pub id: String,
    pub name: String,
    pub target: SyncTarget,
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub color: Option<String>,
    pub notes: String,
    pub proxy_jump: Option<String>,
    pub local_forwards: Vec<String>,
    pub remote_forwards: Vec<String>,
    pub dynamic_forwards: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SyncTarget {
    Alias {
        alias: String,
    },
    Endpoint {
        hostname: String,
        username: Option<String>,
        port: u16,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncGroup {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub parent_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncSnippet {
    pub id: String,
    pub name: String,
    pub content: String,
}

impl SyncFile {
    pub fn from_config(config: &AppConfig) -> Self {
        let mut file = Self {
            version: 1,
            connections: config
                .connections
                .iter()
                .map(SyncConnection::from)
                .collect(),
            groups: config.groups.iter().map(SyncGroup::from).collect(),
            snippets: config.snippets.iter().map(SyncSnippet::from).collect(),
        };
        file.connections.sort_by(|a, b| a.id.cmp(&b.id));
        file.groups.sort_by(|a, b| a.id.cmp(&b.id));
        file.snippets.sort_by(|a, b| a.id.cmp(&b.id));
        file
    }
    pub fn canonical_json(&self) -> Result<Vec<u8>, SyncError> {
        Ok(serde_json::to_vec_pretty(self)?)
    }
    pub fn hash(&self) -> Result<String, SyncError> {
        Ok(format!("{:x}", Sha256::digest(self.canonical_json()?)))
    }
    pub fn apply_to(&self, config: &mut AppConfig) {
        let now = Utc::now();
        config.connections = self
            .connections
            .iter()
            .map(|c| c.to_connection(now))
            .collect();
        config.groups = self.groups.iter().map(SyncGroup::to_group).collect();
        config.snippets = self.snippets.iter().map(SyncSnippet::to_snippet).collect();
    }
}
impl From<&Connection> for SyncConnection {
    fn from(c: &Connection) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            target: match &c.target {
                ConnectionTarget::Alias { alias } => SyncTarget::Alias {
                    alias: alias.clone(),
                },
                ConnectionTarget::Endpoint {
                    hostname,
                    username,
                    port,
                } => SyncTarget::Endpoint {
                    hostname: hostname.clone(),
                    username: username.clone(),
                    port: *port,
                },
            },
            group_id: c.group_id.clone(),
            tags: c.tags.clone(),
            favorite: c.favorite,
            color: c.color.clone(),
            notes: c.notes.clone(),
            proxy_jump: c.proxy_jump.clone(),
            local_forwards: c.local_forwards.clone(),
            remote_forwards: c.remote_forwards.clone(),
            dynamic_forwards: c.dynamic_forwards.clone(),
        }
    }
}
impl SyncConnection {
    fn to_connection(&self, now: chrono::DateTime<Utc>) -> Connection {
        Connection {
            id: self.id.clone(),
            name: self.name.clone(),
            target: match &self.target {
                SyncTarget::Alias { alias } => ConnectionTarget::Alias {
                    alias: alias.clone(),
                },
                SyncTarget::Endpoint {
                    hostname,
                    username,
                    port,
                } => ConnectionTarget::Endpoint {
                    hostname: hostname.clone(),
                    username: username.clone(),
                    port: *port,
                },
            },
            group_id: self.group_id.clone(),
            tags: self.tags.clone(),
            favorite: self.favorite,
            color: self.color.clone(),
            notes: self.notes.clone(),
            proxy_jump: self.proxy_jump.clone(),
            local_forwards: self.local_forwards.clone(),
            remote_forwards: self.remote_forwards.clone(),
            dynamic_forwards: self.dynamic_forwards.clone(),
            remote_command: None,
            local_directories: Vec::new(),
            remote_directories: Vec::new(),
            terminal: Default::default(),
            created_at: now,
            updated_at: now,
        }
    }
}
impl From<&Group> for SyncGroup {
    fn from(g: &Group) -> Self {
        Self {
            id: g.id.clone(),
            name: g.name.clone(),
            color: g.color.clone(),
            parent_id: g.parent_id.clone(),
        }
    }
}
impl SyncGroup {
    fn to_group(&self) -> Group {
        Group {
            id: self.id.clone(),
            name: self.name.clone(),
            color: self.color.clone(),
            parent_id: self.parent_id.clone(),
        }
    }
}
impl From<&CommandSnippet> for SyncSnippet {
    fn from(s: &CommandSnippet) -> Self {
        Self {
            id: s.id.clone(),
            name: s.name.clone(),
            content: s.content.clone(),
        }
    }
}
impl SyncSnippet {
    fn to_snippet(&self) -> CommandSnippet {
        CommandSnippet {
            id: self.id.clone(),
            name: self.name.clone(),
            content: self.content.clone(),
        }
    }
}

pub struct GitSync;
impl GitSync {
    pub fn repository(config: &AppConfig) -> Result<PathBuf, SyncError> {
        config
            .sync
            .repository_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .ok_or(SyncError::NotConfigured)
    }
    pub fn status(config: &AppConfig) -> SyncStatus {
        let Ok(repo) = Self::repository(config) else {
            return SyncStatus::Unconfigured;
        };
        match Self::git(&repo, &["status", "--porcelain"]) {
            Ok(value) if value.trim().is_empty() => SyncStatus::Clean,
            Ok(_) => SyncStatus::LocalChanges,
            Err(_) => SyncStatus::Failed,
        }
    }
    pub fn init(path: &Path, remote: Option<&str>, branch: &str) -> Result<(), SyncError> {
        fs::create_dir_all(path)?;
        Self::git(path, &["init", "-b", branch])?;
        if let Some(url) = remote.filter(|s| !s.trim().is_empty()) {
            if has_embedded_http_credentials(url) {
                return Err(SyncError::EmbeddedCredentials);
            }
            Self::git(path, &["remote", "add", "origin", url])?;
        }
        Ok(())
    }
    pub fn push(config: &mut AppConfig) -> Result<(), SyncError> {
        let repo = Self::repository(config)?;
        let file = SyncFile::from_config(config);
        fs::write(repo.join(SYNC_FILE_NAME), file.canonical_json()?)?;
        Self::git(&repo, &["add", SYNC_FILE_NAME])?;
        let _ = Self::git(&repo, &["commit", "-m", "EasySSH metadata sync"]);
        Self::git(
            &repo,
            &[
                "push",
                "origin",
                config.sync.branch.as_deref().unwrap_or("main"),
            ],
        )?;
        config.sync.last_snapshot_hash = Some(file.hash()?);
        config.sync.last_success_at = Some(Utc::now());
        config.sync.last_error = None;
        Ok(())
    }
    pub fn pull(config: &mut AppConfig) -> Result<bool, SyncError> {
        let repo = Self::repository(config)?;
        Self::git(&repo, &["fetch", "origin"])?;
        let branch = config.sync.branch.clone().unwrap_or_else(|| "main".into());
        let spec = format!("origin/{branch}:{SYNC_FILE_NAME}");
        let bytes = match Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["show", &spec])
            .output()?
        {
            output if output.status.success() => output.stdout,
            output if String::from_utf8_lossy(&output.stderr).contains("does not exist") => {
                return Ok(false)
            }
            output => {
                return Err(SyncError::GitFailed(
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ))
            }
        };
        let file: SyncFile = serde_json::from_slice(&bytes)?;
        let hash = file.hash()?;
        if config
            .sync
            .last_snapshot_hash
            .as_deref()
            .is_some_and(|h| h != hash)
            && SyncFile::from_config(config).hash()?
                != config.sync.last_snapshot_hash.clone().unwrap_or_default()
        {
            return Err(SyncError::GitFailed(
                "local and remote metadata both changed; resolve changes before pulling".into(),
            ));
        }
        file.apply_to(config);
        config.sync.last_snapshot_hash = Some(hash);
        config.sync.last_success_at = Some(Utc::now());
        config.sync.last_error = None;
        Ok(true)
    }
    fn git(repo: &Path, args: &[&str]) -> Result<String, SyncError> {
        let output = Command::new("git")
            .args("-C".split_whitespace())
            .arg(repo)
            .args(args)
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            Err(SyncError::GitFailed(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }
}

fn has_embedded_http_credentials(url: &str) -> bool {
    let Some((scheme, remainder)) = url.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && remainder
            .split('/')
            .next()
            .is_some_and(|authority| authority.contains('@'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Connection;
    #[test]
    fn whitelist_excludes_sensitive_and_local_execution_fields() {
        let mut c = Connection::alias("Production", "prod");
        c.remote_command = Some("danger".into());
        c.local_directories.push("C:/keys".into());
        let mut a = AppConfig::new();
        a.connections.push(c);
        let text = String::from_utf8(SyncFile::from_config(&a).canonical_json().unwrap()).unwrap();
        assert!(!text.contains("remote_command"));
        assert!(!text.contains("local_directories"));
        assert!(!text.contains("terminal"));
        assert!(!text.contains("password"));
    }

    #[test]
    fn rejects_http_urls_with_embedded_credentials() {
        assert!(has_embedded_http_credentials(
            "https://user:secret@example.test/repo.git"
        ));
        assert!(!has_embedded_http_credentials(
            "git@example.test:team/repo.git"
        ));
    }
}
