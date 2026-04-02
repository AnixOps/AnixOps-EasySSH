use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::LiteError;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Self, LiteError> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// 快速初始化 - 仅检查表是否存在，不存在则创建
    pub fn init(&self) -> Result<(), LiteError> {
        // 使用单个事务批量创建表
        self.conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL,
                auth_type TEXT NOT NULL DEFAULT 'agent',
                identity_file TEXT,
                password_encrypted BLOB,
                group_id TEXT,
                status TEXT NOT NULL DEFAULT 'unknown',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS hosts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 22,
                username TEXT NOT NULL,
                auth_type TEXT NOT NULL DEFAULT 'agent',
                identity_file TEXT,
                identity_id TEXT,
                group_id TEXT,
                notes TEXT,
                color TEXT,
                environment TEXT,
                region TEXT,
                purpose TEXT,
                status TEXT NOT NULL DEFAULT 'unknown',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT,
                description TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS host_tags (
                host_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (host_id, tag_id),
                FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS identities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                private_key_path TEXT,
                passphrase_secret_id TEXT,
                auth_type TEXT NOT NULL DEFAULT 'key',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snippets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                description TEXT,
                folder_id TEXT,
                variables_json TEXT,
                scope TEXT NOT NULL DEFAULT 'personal',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                host_id TEXT NOT NULL,
                title TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                last_command TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS layouts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                workspace_mode TEXT NOT NULL,
                layout_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sync_state (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL,
                scope TEXT NOT NULL,
                checkpoint TEXT,
                state_json TEXT,
                last_sync_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                actor TEXT,
                action TEXT NOT NULL,
                target_type TEXT,
                target_id TEXT,
                payload_json TEXT,
                level TEXT NOT NULL DEFAULT 'info',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS remote_desktop_connections (
                id TEXT PRIMARY KEY,
                host_id TEXT NOT NULL,
                name TEXT NOT NULL,
                protocol TEXT NOT NULL DEFAULT 'rdp',
                host TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 3389,
                username TEXT NOT NULL,
                domain TEXT,
                password_encrypted BLOB,
                use_ssh_tunnel INTEGER NOT NULL DEFAULT 0,
                ssh_host TEXT,
                ssh_port INTEGER DEFAULT 22,
                ssh_username TEXT,
                ssh_auth_type TEXT DEFAULT 'agent',
                display_settings_json TEXT,
                performance_settings_json TEXT,
                local_resources_json TEXT,
                experience_settings_json TEXT,
                gateway_settings_json TEXT,
                recording_settings_json TEXT,
                group_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS remote_desktop_sessions (
                id TEXT PRIMARY KEY,
                connection_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'connecting',
                started_at TEXT NOT NULL,
                ended_at TEXT,
                recording_path TEXT,
                recording_active INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (connection_id) REFERENCES remote_desktop_connections(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_hosts_group ON hosts(group_id);
            CREATE INDEX IF NOT EXISTS idx_hosts_name ON hosts(name);
            CREATE INDEX IF NOT EXISTS idx_hosts_status ON hosts(status);
            CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);
            CREATE INDEX IF NOT EXISTS idx_sessions_host ON sessions(host_id);
            CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_events(created_at);
            CREATE INDEX IF NOT EXISTS idx_audit_target ON audit_events(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_rdp_connections_host ON remote_desktop_connections(host_id);
            CREATE INDEX IF NOT EXISTS idx_rdp_connections_group ON remote_desktop_connections(group_id);
            CREATE INDEX IF NOT EXISTS idx_rdp_sessions_connection ON remote_desktop_sessions(connection_id);
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS k8s_clusters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kubeconfig_path TEXT,
                kubeconfig_content TEXT,
                context TEXT NOT NULL,
                server_url TEXT NOT NULL,
                current_namespace TEXT NOT NULL DEFAULT 'default',
                is_connected INTEGER NOT NULL DEFAULT 0,
                last_connected TEXT,
                labels_json TEXT,
                tags_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS k8s_namespaces (
                id TEXT PRIMARY KEY,
                cluster_id TEXT NOT NULL,
                name TEXT NOT NULL,
                status TEXT,
                labels_json TEXT,
                annotations_json TEXT,
                created_at TEXT,
                FOREIGN KEY (cluster_id) REFERENCES k8s_clusters(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS k8s_port_forwards (
                id TEXT PRIMARY KEY,
                cluster_id TEXT NOT NULL,
                namespace TEXT NOT NULL,
                pod_name TEXT,
                service_name TEXT,
                local_port INTEGER NOT NULL,
                remote_port INTEGER NOT NULL,
                protocol TEXT DEFAULT 'TCP',
                is_active INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (cluster_id) REFERENCES k8s_clusters(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_k8s_clusters_name ON k8s_clusters(name);
            CREATE INDEX IF NOT EXISTS idx_k8s_namespaces_cluster ON k8s_namespaces(cluster_id);
            CREATE INDEX IF NOT EXISTS idx_k8s_port_forwards_cluster ON k8s_port_forwards(cluster_id);
            COMMIT;
            "#,
        )?;
        Ok(())
    }

    /// 快速检查数据库是否已初始化（用于启动优化）
    pub fn is_initialized(&self) -> Result<bool, LiteError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('servers', 'groups')",
            [],
            |row| row.get(0),
        )?;
        Ok(count >= 2)
    }

    pub fn get_servers(&self) -> Result<Vec<ServerRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, identity_file, group_id, status, created_at, updated_at FROM servers ORDER BY name"
        )?;

        let servers = stmt
            .query_map([], |row| {
                Ok(ServerRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    auth_type: row.get(5)?,
                    identity_file: row.get(6)?,
                    group_id: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(servers)
    }

    pub fn get_server(&self, id: &str) -> Result<ServerRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, identity_file, group_id, status, created_at, updated_at FROM servers WHERE id = ?"
        )?;

        let server = stmt.query_row([id], |row| {
            Ok(ServerRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_type: row.get(5)?,
                identity_file: row.get(6)?,
                group_id: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        Ok(server)
    }

    pub fn add_server(&self, server: &NewServer) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO servers (id, name, host, port, username, auth_type, identity_file, group_id, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                server.id,
                server.name,
                server.host,
                server.port,
                server.username,
                server.auth_type,
                server.identity_file,
                server.group_id,
                server.status.as_str(),
                now,
                now
            ],
        )?;
        Ok(())
    }

    pub fn update_server(&self, server: &UpdateServer) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE servers SET name = ?, host = ?, port = ?, username = ?, auth_type = ?, identity_file = ?, group_id = ?, status = ?, updated_at = ? WHERE id = ?",
            params![
                server.name,
                server.host,
                server.port,
                server.username,
                server.auth_type,
                server.identity_file,
                server.group_id,
                server.status.as_str(),
                now,
                server.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_server(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM servers WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_groups(&self) -> Result<Vec<GroupRecord>, LiteError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at, updated_at FROM groups ORDER BY name")?;

        let groups = stmt
            .query_map([], |row| {
                Ok(GroupRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    pub fn add_group(&self, group: &NewGroup) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO groups (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
            params![group.id, group.name, now, now],
        )?;
        Ok(())
    }

    pub fn update_group(&self, group: &UpdateGroup) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE groups SET name = ?, updated_at = ? WHERE id = ?",
            params![group.name, now, group.id],
        )?;
        Ok(())
    }

    pub fn delete_group(&self, id: &str) -> Result<(), LiteError> {
        self.conn.execute("DELETE FROM groups WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_hosts(&self) -> Result<Vec<HostRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, identity_file, identity_id, group_id, notes, color, environment, region, purpose, status, created_at, updated_at FROM hosts ORDER BY name",
        )?;

        let hosts = stmt
            .query_map([], |row| {
                Ok(HostRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    host: row.get(2)?,
                    port: row.get(3)?,
                    username: row.get(4)?,
                    auth_type: row.get(5)?,
                    identity_file: row.get(6)?,
                    identity_id: row.get(7)?,
                    group_id: row.get(8)?,
                    notes: row.get(9)?,
                    color: row.get(10)?,
                    environment: row.get(11)?,
                    region: row.get(12)?,
                    purpose: row.get(13)?,
                    status: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(hosts)
    }

    pub fn get_host(&self, id: &str) -> Result<HostRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, host, port, username, auth_type, identity_file, identity_id, group_id, notes, color, environment, region, purpose, status, created_at, updated_at FROM hosts WHERE id = ?",
        )?;

        let host = stmt.query_row([id], |row| {
            Ok(HostRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_type: row.get(5)?,
                identity_file: row.get(6)?,
                identity_id: row.get(7)?,
                group_id: row.get(8)?,
                notes: row.get(9)?,
                color: row.get(10)?,
                environment: row.get(11)?,
                region: row.get(12)?,
                purpose: row.get(13)?,
                status: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;

        Ok(host)
    }

    pub fn add_host(&self, host: &NewHost) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO hosts (id, name, host, port, username, auth_type, identity_file, identity_id, group_id, notes, color, environment, region, purpose, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                host.id,
                host.name,
                host.host,
                host.port,
                host.username,
                host.auth_type,
                host.identity_file,
                host.identity_id,
                host.group_id,
                host.notes,
                host.color,
                host.environment,
                host.region,
                host.purpose,
                host.status,
                now,
                now
            ],
        )?;
        Ok(())
    }

    pub fn update_host(&self, host: &UpdateHost) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE hosts SET name = ?, host = ?, port = ?, username = ?, auth_type = ?, identity_file = ?, identity_id = ?, group_id = ?, notes = ?, color = ?, environment = ?, region = ?, purpose = ?, status = ?, updated_at = ? WHERE id = ?",
            params![
                host.name,
                host.host,
                host.port,
                host.username,
                host.auth_type,
                host.identity_file,
                host.identity_id,
                host.group_id,
                host.notes,
                host.color,
                host.environment,
                host.region,
                host.purpose,
                host.status,
                now,
                host.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_host(&self, id: &str) -> Result<(), LiteError> {
        self.conn.execute("DELETE FROM hosts WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_tags(&self) -> Result<Vec<TagRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, description, created_at, updated_at FROM tags ORDER BY name",
        )?;

        let tags = stmt
            .query_map([], |row| {
                Ok(TagRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub fn get_tag(&self, id: &str) -> Result<TagRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, description, created_at, updated_at FROM tags WHERE id = ?",
        )?;

        let tag = stmt.query_row([id], |row| {
            Ok(TagRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        Ok(tag)
    }

    pub fn add_tag(&self, tag: &NewTag) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO tags (id, name, color, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![tag.id, tag.name, tag.color, tag.description, now, now],
        )?;
        Ok(())
    }

    pub fn update_tag(&self, tag: &UpdateTag) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE tags SET name = ?, color = ?, description = ?, updated_at = ? WHERE id = ?",
            params![tag.name, tag.color, tag.description, now, tag.id],
        )?;
        Ok(())
    }

    pub fn delete_tag(&self, id: &str) -> Result<(), LiteError> {
        self.conn.execute("DELETE FROM tags WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_host_tags(&self, host_id: &str) -> Result<Vec<TagRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.color, t.description, t.created_at, t.updated_at FROM tags t INNER JOIN host_tags ht ON ht.tag_id = t.id WHERE ht.host_id = ? ORDER BY t.name",
        )?;

        let tags = stmt
            .query_map([host_id], |row| {
                Ok(TagRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    description: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tags)
    }

    pub fn set_host_tag(&self, host_id: &str, tag_id: &str) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO host_tags (host_id, tag_id, created_at) VALUES (?, ?, ?)",
            params![host_id, tag_id, now],
        )?;
        Ok(())
    }

    pub fn remove_host_tag(&self, host_id: &str, tag_id: &str) -> Result<(), LiteError> {
        self.conn.execute(
            "DELETE FROM host_tags WHERE host_id = ? AND tag_id = ?",
            params![host_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_identities(&self) -> Result<Vec<IdentityRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, private_key_path, passphrase_secret_id, auth_type, created_at, updated_at FROM identities ORDER BY name",
        )?;

        let identities = stmt
            .query_map([], |row| {
                Ok(IdentityRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    private_key_path: row.get(2)?,
                    passphrase_secret_id: row.get(3)?,
                    auth_type: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(identities)
    }

    pub fn get_identity(&self, id: &str) -> Result<IdentityRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, private_key_path, passphrase_secret_id, auth_type, created_at, updated_at FROM identities WHERE id = ?",
        )?;

        let identity = stmt.query_row([id], |row| {
            Ok(IdentityRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                private_key_path: row.get(2)?,
                passphrase_secret_id: row.get(3)?,
                auth_type: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        Ok(identity)
    }

    pub fn add_identity(&self, identity: &NewIdentity) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO identities (id, name, private_key_path, passphrase_secret_id, auth_type, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![identity.id, identity.name, identity.private_key_path, identity.passphrase_secret_id, identity.auth_type, now, now],
        )?;
        Ok(())
    }

    pub fn update_identity(&self, identity: &UpdateIdentity) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE identities SET name = ?, private_key_path = ?, passphrase_secret_id = ?, auth_type = ?, updated_at = ? WHERE id = ?",
            params![identity.name, identity.private_key_path, identity.passphrase_secret_id, identity.auth_type, now, identity.id],
        )?;
        Ok(())
    }

    pub fn delete_identity(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM identities WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_snippets(&self) -> Result<Vec<SnippetRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, description, folder_id, variables_json, scope, created_at, updated_at FROM snippets ORDER BY name",
        )?;

        let snippets = stmt
            .query_map([], |row| {
                Ok(SnippetRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    folder_id: row.get(4)?,
                    variables_json: row.get(5)?,
                    scope: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(snippets)
    }

    pub fn get_snippet(&self, id: &str) -> Result<SnippetRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, command, description, folder_id, variables_json, scope, created_at, updated_at FROM snippets WHERE id = ?",
        )?;

        let snippet = stmt.query_row([id], |row| {
            Ok(SnippetRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                command: row.get(2)?,
                description: row.get(3)?,
                folder_id: row.get(4)?,
                variables_json: row.get(5)?,
                scope: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        Ok(snippet)
    }

    pub fn add_snippet(&self, snippet: &NewSnippet) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO snippets (id, name, command, description, folder_id, variables_json, scope, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![snippet.id, snippet.name, snippet.command, snippet.description, snippet.folder_id, snippet.variables_json, snippet.scope, now, now],
        )?;
        Ok(())
    }

    pub fn update_snippet(&self, snippet: &UpdateSnippet) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE snippets SET name = ?, command = ?, description = ?, folder_id = ?, variables_json = ?, scope = ?, updated_at = ? WHERE id = ?",
            params![snippet.name, snippet.command, snippet.description, snippet.folder_id, snippet.variables_json, snippet.scope, now, snippet.id],
        )?;
        Ok(())
    }

    pub fn delete_snippet(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM snippets WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_sessions(&self) -> Result<Vec<SessionRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, host_id, title, status, last_command, started_at, ended_at FROM sessions ORDER BY started_at DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    host_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    last_command: row.get(4)?,
                    started_at: row.get(5)?,
                    ended_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn get_session(&self, id: &str) -> Result<SessionRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, host_id, title, status, last_command, started_at, ended_at FROM sessions WHERE id = ?",
        )?;

        let session = stmt.query_row([id], |row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                host_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                last_command: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
            })
        })?;

        Ok(session)
    }

    pub fn add_session(&self, session: &NewSession) -> Result<(), LiteError> {
        self.conn.execute(
            "INSERT INTO sessions (id, host_id, title, status, last_command, started_at, ended_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![session.id, session.host_id, session.title, session.status, session.last_command, session.started_at, session.ended_at],
        )?;
        Ok(())
    }

    pub fn update_session(&self, session: &UpdateSession) -> Result<(), LiteError> {
        self.conn.execute(
            "UPDATE sessions SET host_id = ?, title = ?, status = ?, last_command = ?, started_at = ?, ended_at = ? WHERE id = ?",
            params![session.host_id, session.title, session.status, session.last_command, session.started_at, session.ended_at, session.id],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM sessions WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_layouts(&self) -> Result<Vec<LayoutRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, workspace_mode, layout_json, created_at, updated_at FROM layouts ORDER BY updated_at DESC",
        )?;

        let layouts = stmt
            .query_map([], |row| {
                Ok(LayoutRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    workspace_mode: row.get(2)?,
                    layout_json: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(layouts)
    }

    pub fn get_layout(&self, id: &str) -> Result<LayoutRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, workspace_mode, layout_json, created_at, updated_at FROM layouts WHERE id = ?",
        )?;

        let layout = stmt.query_row([id], |row| {
            Ok(LayoutRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                workspace_mode: row.get(2)?,
                layout_json: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        Ok(layout)
    }

    pub fn add_layout(&self, layout: &NewLayout) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO layouts (id, name, workspace_mode, layout_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![layout.id, layout.name, layout.workspace_mode, layout.layout_json, now, now],
        )?;
        Ok(())
    }

    pub fn update_layout(&self, layout: &UpdateLayout) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE layouts SET name = ?, workspace_mode = ?, layout_json = ?, updated_at = ? WHERE id = ?",
            params![layout.name, layout.workspace_mode, layout.layout_json, now, layout.id],
        )?;
        Ok(())
    }

    pub fn delete_layout(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM layouts WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_sync_states(&self) -> Result<Vec<SyncStateRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, device_id, scope, checkpoint, state_json, last_sync_at, created_at, updated_at FROM sync_state ORDER BY updated_at DESC",
        )?;

        let states = stmt
            .query_map([], |row| {
                Ok(SyncStateRecord {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    scope: row.get(2)?,
                    checkpoint: row.get(3)?,
                    state_json: row.get(4)?,
                    last_sync_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(states)
    }

    pub fn get_sync_state(&self, id: &str) -> Result<SyncStateRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, device_id, scope, checkpoint, state_json, last_sync_at, created_at, updated_at FROM sync_state WHERE id = ?",
        )?;

        let state = stmt.query_row([id], |row| {
            Ok(SyncStateRecord {
                id: row.get(0)?,
                device_id: row.get(1)?,
                scope: row.get(2)?,
                checkpoint: row.get(3)?,
                state_json: row.get(4)?,
                last_sync_at: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        Ok(state)
    }

    pub fn upsert_sync_state(&self, state: &NewSyncState) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO sync_state (id, device_id, scope, checkpoint, state_json, last_sync_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, COALESCE((SELECT created_at FROM sync_state WHERE id = ?), ?), ?)",
            params![state.id, state.device_id, state.scope, state.checkpoint, state.state_json, state.last_sync_at, state.id, now, now],
        )?;
        Ok(())
    }

    pub fn delete_sync_state(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM sync_state WHERE id = ?", [id])?;
        Ok(())
    }

    pub fn get_audit_events(&self) -> Result<Vec<AuditEventRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, actor, action, target_type, target_id, payload_json, level, created_at FROM audit_events ORDER BY created_at DESC",
        )?;

        let events = stmt
            .query_map([], |row| {
                Ok(AuditEventRecord {
                    id: row.get(0)?,
                    actor: row.get(1)?,
                    action: row.get(2)?,
                    target_type: row.get(3)?,
                    target_id: row.get(4)?,
                    payload_json: row.get(5)?,
                    level: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    pub fn add_audit_event(&self, event: &NewAuditEvent) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO audit_events (id, actor, action, target_type, target_id, payload_json, level, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![event.id, event.actor, event.action, event.target_type, event.target_id, event.payload_json, event.level, now],
        )?;
        Ok(())
    }

    pub fn get_config(&self, key: &str) -> Result<Option<String>, LiteError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM config WHERE key = ?")?;
        let result = stmt.query_row([key], |row| row.get(0)).ok();
        Ok(result)
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<(), LiteError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    // ============ Remote Desktop Connection Methods ============

    pub fn get_remote_desktop_connections(
        &self,
    ) -> Result<Vec<RemoteDesktopConnectionRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, host_id, name, protocol, host, port, username, domain, password_encrypted,
                    use_ssh_tunnel, ssh_host, ssh_port, ssh_username, ssh_auth_type,
                    display_settings_json, performance_settings_json, local_resources_json,
                    experience_settings_json, gateway_settings_json, recording_settings_json,
                    group_id, created_at, updated_at
             FROM remote_desktop_connections ORDER BY name",
        )?;

        let connections = stmt
            .query_map([], |row| {
                Ok(RemoteDesktopConnectionRecord {
                    id: row.get(0)?,
                    host_id: row.get(1)?,
                    name: row.get(2)?,
                    protocol: row.get(3)?,
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    domain: row.get(7)?,
                    password_encrypted: row.get(8)?,
                    use_ssh_tunnel: row.get::<_, i64>(9)? != 0,
                    ssh_host: row.get(10)?,
                    ssh_port: row.get(11)?,
                    ssh_username: row.get(12)?,
                    ssh_auth_type: row.get(13)?,
                    display_settings_json: row.get(14)?,
                    performance_settings_json: row.get(15)?,
                    local_resources_json: row.get(16)?,
                    experience_settings_json: row.get(17)?,
                    gateway_settings_json: row.get(18)?,
                    recording_settings_json: row.get(19)?,
                    group_id: row.get(20)?,
                    created_at: row.get(21)?,
                    updated_at: row.get(22)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(connections)
    }

    pub fn get_remote_desktop_connection(
        &self,
        id: &str,
    ) -> Result<RemoteDesktopConnectionRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, host_id, name, protocol, host, port, username, domain, password_encrypted,
                    use_ssh_tunnel, ssh_host, ssh_port, ssh_username, ssh_auth_type,
                    display_settings_json, performance_settings_json, local_resources_json,
                    experience_settings_json, gateway_settings_json, recording_settings_json,
                    group_id, created_at, updated_at
             FROM remote_desktop_connections WHERE id = ?",
        )?;

        let connection = stmt.query_row([id], |row| {
            Ok(RemoteDesktopConnectionRecord {
                id: row.get(0)?,
                host_id: row.get(1)?,
                name: row.get(2)?,
                protocol: row.get(3)?,
                host: row.get(4)?,
                port: row.get(5)?,
                username: row.get(6)?,
                domain: row.get(7)?,
                password_encrypted: row.get(8)?,
                use_ssh_tunnel: row.get::<_, i64>(9)? != 0,
                ssh_host: row.get(10)?,
                ssh_port: row.get(11)?,
                ssh_username: row.get(12)?,
                ssh_auth_type: row.get(13)?,
                display_settings_json: row.get(14)?,
                performance_settings_json: row.get(15)?,
                local_resources_json: row.get(16)?,
                experience_settings_json: row.get(17)?,
                gateway_settings_json: row.get(18)?,
                recording_settings_json: row.get(19)?,
                group_id: row.get(20)?,
                created_at: row.get(21)?,
                updated_at: row.get(22)?,
            })
        })?;

        Ok(connection)
    }

    pub fn add_remote_desktop_connection(
        &self,
        connection: &NewRemoteDesktopConnection,
    ) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "INSERT INTO remote_desktop_connections
             (id, host_id, name, protocol, host, port, username, domain, password_encrypted,
              use_ssh_tunnel, ssh_host, ssh_port, ssh_username, ssh_auth_type,
              display_settings_json, performance_settings_json, local_resources_json,
              experience_settings_json, gateway_settings_json, recording_settings_json,
              group_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                connection.id,
                connection.host_id,
                connection.name,
                connection.protocol,
                connection.host,
                connection.port,
                connection.username,
                connection.domain,
                connection.password_encrypted,
                connection.use_ssh_tunnel as i64,
                connection.ssh_host,
                connection.ssh_port,
                connection.ssh_username,
                connection.ssh_auth_type,
                connection.display_settings_json,
                connection.performance_settings_json,
                connection.local_resources_json,
                connection.experience_settings_json,
                connection.gateway_settings_json,
                connection.recording_settings_json,
                connection.group_id,
                now,
                now
            ],
        )?;
        Ok(())
    }

    pub fn update_remote_desktop_connection(
        &self,
        connection: &UpdateRemoteDesktopConnection,
    ) -> Result<(), LiteError> {
        let now = chrono_now();
        self.conn.execute(
            "UPDATE remote_desktop_connections SET
                host_id = ?, name = ?, protocol = ?, host = ?, port = ?, username = ?,
                domain = ?, password_encrypted = ?, use_ssh_tunnel = ?, ssh_host = ?,
                ssh_port = ?, ssh_username = ?, ssh_auth_type = ?, display_settings_json = ?,
                performance_settings_json = ?, local_resources_json = ?, experience_settings_json = ?,
                gateway_settings_json = ?, recording_settings_json = ?, group_id = ?, updated_at = ?
             WHERE id = ?",
            params![
                connection.host_id,
                connection.name,
                connection.protocol,
                connection.host,
                connection.port,
                connection.username,
                connection.domain,
                connection.password_encrypted,
                connection.use_ssh_tunnel as i64,
                connection.ssh_host,
                connection.ssh_port,
                connection.ssh_username,
                connection.ssh_auth_type,
                connection.display_settings_json,
                connection.performance_settings_json,
                connection.local_resources_json,
                connection.experience_settings_json,
                connection.gateway_settings_json,
                connection.recording_settings_json,
                connection.group_id,
                now,
                connection.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_remote_desktop_connection(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM remote_desktop_connections WHERE id = ?", [id])?;
        Ok(())
    }

    // ============ Remote Desktop Session Methods ============

    pub fn get_remote_desktop_sessions(
        &self,
    ) -> Result<Vec<RemoteDesktopSessionRecord>, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, connection_id, status, started_at, ended_at, recording_path, recording_active
             FROM remote_desktop_sessions ORDER BY started_at DESC"
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(RemoteDesktopSessionRecord {
                    id: row.get(0)?,
                    connection_id: row.get(1)?,
                    status: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    recording_path: row.get(5)?,
                    recording_active: row.get::<_, i64>(6)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn get_remote_desktop_session(
        &self,
        id: &str,
    ) -> Result<RemoteDesktopSessionRecord, LiteError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, connection_id, status, started_at, ended_at, recording_path, recording_active
             FROM remote_desktop_sessions WHERE id = ?"
        )?;

        let session = stmt.query_row([id], |row| {
            Ok(RemoteDesktopSessionRecord {
                id: row.get(0)?,
                connection_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                recording_path: row.get(5)?,
                recording_active: row.get::<_, i64>(6)? != 0,
            })
        })?;

        Ok(session)
    }

    pub fn add_remote_desktop_session(
        &self,
        session: &NewRemoteDesktopSession,
    ) -> Result<(), LiteError> {
        self.conn.execute(
            "INSERT INTO remote_desktop_sessions
             (id, connection_id, status, started_at, ended_at, recording_path, recording_active)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id,
                session.connection_id,
                session.status,
                session.started_at,
                session.ended_at,
                session.recording_path,
                session.recording_active as i64
            ],
        )?;
        Ok(())
    }

    pub fn update_remote_desktop_session(
        &self,
        session: &UpdateRemoteDesktopSession,
    ) -> Result<(), LiteError> {
        self.conn.execute(
            "UPDATE remote_desktop_sessions SET
                connection_id = ?, status = ?, started_at = ?, ended_at = ?,
                recording_path = ?, recording_active = ?
             WHERE id = ?",
            params![
                session.connection_id,
                session.status,
                session.started_at,
                session.ended_at,
                session.recording_path,
                session.recording_active as i64,
                session.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_remote_desktop_session(&self, id: &str) -> Result<(), LiteError> {
        self.conn
            .execute("DELETE FROM remote_desktop_sessions WHERE id = ?", [id])?;
        Ok(())
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

#[derive(serde::Serialize, Clone)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewServer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
    pub status: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateServer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
    pub status: String,
}

#[derive(serde::Serialize)]
pub struct GroupRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewGroup {
    pub id: String,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateGroup {
    pub id: String,
    pub name: String,
}

#[derive(serde::Serialize, Clone)]
pub struct HostRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub identity_id: Option<String>,
    pub group_id: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub environment: Option<String>,
    pub region: Option<String>,
    pub purpose: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub identity_id: Option<String>,
    pub group_id: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub environment: Option<String>,
    pub region: Option<String>,
    pub purpose: Option<String>,
    pub status: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub identity_id: Option<String>,
    pub group_id: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub environment: Option<String>,
    pub region: Option<String>,
    pub purpose: Option<String>,
    pub status: String,
}

#[derive(serde::Serialize)]
pub struct TagRecord {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewTag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateTag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(serde::Serialize)]
pub struct IdentityRecord {
    pub id: String,
    pub name: String,
    pub private_key_path: Option<String>,
    pub passphrase_secret_id: Option<String>,
    pub auth_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewIdentity {
    pub id: String,
    pub name: String,
    pub private_key_path: Option<String>,
    pub passphrase_secret_id: Option<String>,
    pub auth_type: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateIdentity {
    pub id: String,
    pub name: String,
    pub private_key_path: Option<String>,
    pub passphrase_secret_id: Option<String>,
    pub auth_type: String,
}

#[derive(serde::Serialize)]
pub struct SnippetRecord {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub variables_json: Option<String>,
    pub scope: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewSnippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub variables_json: Option<String>,
    pub scope: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateSnippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub folder_id: Option<String>,
    pub variables_json: Option<String>,
    pub scope: String,
}

#[derive(serde::Serialize)]
pub struct SessionRecord {
    pub id: String,
    pub host_id: String,
    pub title: Option<String>,
    pub status: String,
    pub last_command: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct NewSession {
    pub id: String,
    pub host_id: String,
    pub title: Option<String>,
    pub status: String,
    pub last_command: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateSession {
    pub id: String,
    pub host_id: String,
    pub title: Option<String>,
    pub status: String,
    pub last_command: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(serde::Serialize)]
pub struct LayoutRecord {
    pub id: String,
    pub name: String,
    pub workspace_mode: String,
    pub layout_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewLayout {
    pub id: String,
    pub name: String,
    pub workspace_mode: String,
    pub layout_json: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateLayout {
    pub id: String,
    pub name: String,
    pub workspace_mode: String,
    pub layout_json: String,
}

#[derive(serde::Serialize)]
pub struct SyncStateRecord {
    pub id: String,
    pub device_id: String,
    pub scope: String,
    pub checkpoint: Option<String>,
    pub state_json: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewSyncState {
    pub id: String,
    pub device_id: String,
    pub scope: String,
    pub checkpoint: Option<String>,
    pub state_json: Option<String>,
    pub last_sync_at: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateSyncState {
    pub id: String,
    pub device_id: String,
    pub scope: String,
    pub checkpoint: Option<String>,
    pub state_json: Option<String>,
    pub last_sync_at: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AuditEventRecord {
    pub id: String,
    pub actor: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub payload_json: Option<String>,
    pub level: String,
    pub created_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewAuditEvent {
    pub id: String,
    pub actor: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub payload_json: Option<String>,
    pub level: String,
}

#[derive(serde::Serialize, Clone)]
pub struct RemoteDesktopConnectionRecord {
    pub id: String,
    pub host_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub domain: Option<String>,
    pub password_encrypted: Option<Vec<u8>>,
    pub use_ssh_tunnel: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_username: Option<String>,
    pub ssh_auth_type: Option<String>,
    pub display_settings_json: Option<String>,
    pub performance_settings_json: Option<String>,
    pub local_resources_json: Option<String>,
    pub experience_settings_json: Option<String>,
    pub gateway_settings_json: Option<String>,
    pub recording_settings_json: Option<String>,
    pub group_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
pub struct NewRemoteDesktopConnection {
    pub id: String,
    pub host_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub domain: Option<String>,
    pub password_encrypted: Option<Vec<u8>>,
    pub use_ssh_tunnel: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_username: Option<String>,
    pub ssh_auth_type: Option<String>,
    pub display_settings_json: Option<String>,
    pub performance_settings_json: Option<String>,
    pub local_resources_json: Option<String>,
    pub experience_settings_json: Option<String>,
    pub gateway_settings_json: Option<String>,
    pub recording_settings_json: Option<String>,
    pub group_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UpdateRemoteDesktopConnection {
    pub id: String,
    pub host_id: String,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub domain: Option<String>,
    pub password_encrypted: Option<Vec<u8>>,
    pub use_ssh_tunnel: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i64>,
    pub ssh_username: Option<String>,
    pub ssh_auth_type: Option<String>,
    pub display_settings_json: Option<String>,
    pub performance_settings_json: Option<String>,
    pub local_resources_json: Option<String>,
    pub experience_settings_json: Option<String>,
    pub gateway_settings_json: Option<String>,
    pub recording_settings_json: Option<String>,
    pub group_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct RemoteDesktopSessionRecord {
    pub id: String,
    pub connection_id: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub recording_path: Option<String>,
    pub recording_active: bool,
}

#[derive(serde::Deserialize)]
pub struct NewRemoteDesktopSession {
    pub id: String,
    pub connection_id: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub recording_path: Option<String>,
    pub recording_active: bool,
}

#[derive(serde::Deserialize)]
pub struct UpdateRemoteDesktopSession {
    pub id: String,
    pub connection_id: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub recording_path: Option<String>,
    pub recording_active: bool,
}

/// 全局数据库连接
pub static DATABASE: Mutex<Option<Database>> = Mutex::new(None);

/// 获取数据库路径
pub fn get_db_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("easyssh-lite");

    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("easyssh.db")
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_db_path() {
        let path = get_db_path();
        assert!(path.to_str().unwrap().contains("easyssh.db"));
    }

    #[test]
    fn test_chrono_now_format() {
        let now = chrono_now();
        // 应该是一个数字字符串
        assert!(now.parse::<u64>().is_ok());
    }

    #[test]
    fn test_server_record_serialization() {
        let record = ServerRecord {
            id: "test-id".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            group_id: Some("group-1".to_string()),
            status: "online".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("Test Server"));
        assert!(json.contains("192.168.1.1"));
    }

    #[test]
    fn test_group_record_serialization() {
        let record = GroupRecord {
            id: "group-id".to_string(),
            name: "Test Group".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("Test Group"));
    }

    #[test]
    fn test_new_server_deserialization() {
        let json = r#"{
            "id": "srv-1",
            "name": "My Server",
            "host": "10.0.0.1",
            "port": 2222,
            "username": "root",
            "auth_type": "key",
            "identity_file": "/path/to/key",
            "group_id": null,
            "status": "online"
        }"#;
        let server: NewServer = serde_json::from_str(json).unwrap();
        assert_eq!(server.name, "My Server");
        assert_eq!(server.port, 2222);
        assert!(server.identity_file.is_some());
    }

    #[test]
    fn test_new_group_deserialization() {
        let json = r#"{"id": "grp-1", "name": "Production"}"#;
        let group: NewGroup = serde_json::from_str(json).unwrap();
        assert_eq!(group.name, "Production");
    }

    #[test]
    fn test_database_init_and_is_initialized() {
        // 创建临时数据库文件
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        // 初始状态：未初始化
        {
            let db = Database::new(db_path.clone()).unwrap();
            assert!(!db.is_initialized().unwrap());

            // 初始化后
            db.init().unwrap();
            assert!(db.is_initialized().unwrap());
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_server_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_server_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建分组
            let group = NewGroup {
                id: "grp-1".to_string(),
                name: "Test Group".to_string(),
            };
            db.add_group(&group).unwrap();

            // 创建服务器
            let server = NewServer {
                id: "srv-1".to_string(),
                name: "Test Server".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_id: Some("grp-1".to_string()),
                status: "online".to_string(),
            };
            db.add_server(&server).unwrap();

            // 读取并验证
            let servers = db.get_servers().unwrap();
            assert_eq!(servers.len(), 1);
            assert_eq!(servers[0].name, "Test Server");
            assert_eq!(servers[0].host, "192.168.1.100");

            // 更新服务器
            let update = UpdateServer {
                id: "srv-1".to_string(),
                name: "Updated Server".to_string(),
                host: "192.168.1.200".to_string(),
                port: 2222,
                username: "root".to_string(),
                auth_type: "key".to_string(),
                identity_file: Some("/path/to/key".to_string()),
                group_id: None,
                status: "offline".to_string(),
            };
            db.update_server(&update).unwrap();

            // 验证更新
            let server = db.get_server("srv-1").unwrap();
            assert_eq!(server.name, "Updated Server");
            assert_eq!(server.port, 2222);

            // 删除服务器
            db.delete_server("srv-1").unwrap();
            let servers = db.get_servers().unwrap();
            assert_eq!(servers.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_group_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_group_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建分组
            let group1 = NewGroup {
                id: "grp-1".to_string(),
                name: "Development".to_string(),
            };
            let group2 = NewGroup {
                id: "grp-2".to_string(),
                name: "Production".to_string(),
            };
            db.add_group(&group1).unwrap();
            db.add_group(&group2).unwrap();

            // 读取所有分组
            let groups = db.get_groups().unwrap();
            assert_eq!(groups.len(), 2);

            // 更新分组
            let update = UpdateGroup {
                id: "grp-1".to_string(),
                name: "Dev Team".to_string(),
            };
            db.update_group(&update).unwrap();
            let groups = db.get_groups().unwrap();
            let grp = groups.iter().find(|g| g.id == "grp-1").unwrap();
            assert_eq!(grp.name, "Dev Team");

            // 删除分组
            db.delete_group("grp-2").unwrap();
            let groups = db.get_groups().unwrap();
            assert_eq!(groups.len(), 1);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_config_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_config_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 初始无配置
            assert!(db.get_config("theme").unwrap().is_none());

            // 设置配置
            db.set_config("theme", "dark").unwrap();
            assert_eq!(db.get_config("theme").unwrap().unwrap(), "dark");

            // 更新配置
            db.set_config("theme", "light").unwrap();
            assert_eq!(db.get_config("theme").unwrap().unwrap(), "light");

            // 多配置键
            db.set_config("language", "zh-CN").unwrap();
            assert_eq!(db.get_config("language").unwrap().unwrap(), "zh-CN");
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_session_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_session_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 首先创建host，因为session有外键约束
            let host = NewHost {
                id: "host-1".to_string(),
                name: "Test Host".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "online".to_string(),
            };
            db.add_host(&host).unwrap();

            // 创建会话
            let session = NewSession {
                id: "sess-1".to_string(),
                host_id: "host-1".to_string(),
                title: Some("Test Session".to_string()),
                status: "active".to_string(),
                last_command: Some("ls -la".to_string()),
                started_at: chrono_now(),
                ended_at: None,
            };
            db.add_session(&session).unwrap();

            // 读取并验证
            let sessions = db.get_sessions().unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].title, Some("Test Session".to_string()));

            // 更新会话
            let update = UpdateSession {
                id: "sess-1".to_string(),
                host_id: "host-1".to_string(),
                title: Some("Updated Session".to_string()),
                status: "closed".to_string(),
                last_command: Some("pwd".to_string()),
                started_at: chrono_now(),
                ended_at: Some(chrono_now()),
            };
            db.update_session(&update).unwrap();

            // 验证更新
            let session = db.get_session("sess-1").unwrap();
            assert_eq!(session.status, "closed");

            // 删除会话
            db.delete_session("sess-1").unwrap();
            let sessions = db.get_sessions().unwrap();
            assert_eq!(sessions.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_snippet_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_snippet_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建代码片段
            let snippet = NewSnippet {
                id: "snip-1".to_string(),
                name: "List Files".to_string(),
                command: "ls -la".to_string(),
                description: Some("List all files".to_string()),
                folder_id: Some("folder-1".to_string()),
                variables_json: Some("[]".to_string()),
                scope: "personal".to_string(),
            };
            db.add_snippet(&snippet).unwrap();

            // 读取并验证
            let snippets = db.get_snippets().unwrap();
            assert_eq!(snippets.len(), 1);
            assert_eq!(snippets[0].name, "List Files");

            // 更新代码片段
            let update = UpdateSnippet {
                id: "snip-1".to_string(),
                name: "List Files Updated".to_string(),
                command: "ls -lh".to_string(),
                description: Some("List files with human sizes".to_string()),
                folder_id: None,
                variables_json: None,
                scope: "shared".to_string(),
            };
            db.update_snippet(&update).unwrap();

            // 验证更新
            let snippet = db.get_snippet("snip-1").unwrap();
            assert_eq!(snippet.name, "List Files Updated");
            assert_eq!(snippet.scope, "shared");

            // 删除代码片段
            db.delete_snippet("snip-1").unwrap();
            let snippets = db.get_snippets().unwrap();
            assert_eq!(snippets.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_tag_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_tag_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建标签
            let tag = NewTag {
                id: "tag-1".to_string(),
                name: "Production".to_string(),
                color: Some("#FF0000".to_string()),
                description: Some("Production servers".to_string()),
            };
            db.add_tag(&tag).unwrap();

            // 读取并验证
            let tags = db.get_tags().unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].name, "Production");
            assert_eq!(tags[0].color, Some("#FF0000".to_string()));

            // 更新标签
            let update = UpdateTag {
                id: "tag-1".to_string(),
                name: "Prod".to_string(),
                color: Some("#FF5733".to_string()),
                description: Some("Prod servers".to_string()),
            };
            db.update_tag(&update).unwrap();

            // 验证更新
            let tag = db.get_tag("tag-1").unwrap();
            assert_eq!(tag.name, "Prod");

            // 删除标签
            db.delete_tag("tag-1").unwrap();
            let tags = db.get_tags().unwrap();
            assert_eq!(tags.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_audit_event_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_audit_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建审计事件
            let event = NewAuditEvent {
                id: "audit-1".to_string(),
                actor: Some("user-1".to_string()),
                action: "server.connect".to_string(),
                target_type: Some("server".to_string()),
                target_id: Some("srv-1".to_string()),
                payload_json: Some(r#"{"ip": "192.168.1.1"}"#.to_string()),
                level: "info".to_string(),
            };
            db.add_audit_event(&event).unwrap();

            // 读取并验证
            let events = db.get_audit_events().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].action, "server.connect");
            assert_eq!(events[0].actor, Some("user-1".to_string()));
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_layout_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_layout_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建布局
            let layout = NewLayout {
                id: "layout-1".to_string(),
                name: "Default".to_string(),
                workspace_mode: "standard".to_string(),
                layout_json: r#"{"split": "horizontal"}"#.to_string(),
            };
            db.add_layout(&layout).unwrap();

            // 读取并验证
            let layouts = db.get_layouts().unwrap();
            assert_eq!(layouts.len(), 1);
            assert_eq!(layouts[0].name, "Default");

            // 更新布局
            let update = UpdateLayout {
                id: "layout-1".to_string(),
                name: "Custom".to_string(),
                workspace_mode: "developer".to_string(),
                layout_json: r#"{"split": "vertical"}"#.to_string(),
            };
            db.update_layout(&update).unwrap();

            // 验证更新
            let layout = db.get_layout("layout-1").unwrap();
            assert_eq!(layout.name, "Custom");

            // 删除布局
            db.delete_layout("layout-1").unwrap();
            let layouts = db.get_layouts().unwrap();
            assert_eq!(layouts.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_sync_state_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_sync_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // 创建同步状态
            let state = NewSyncState {
                id: "sync-1".to_string(),
                device_id: "device-1".to_string(),
                scope: "full".to_string(),
                checkpoint: Some("checkpoint-abc".to_string()),
                state_json: Some(r#"{"last_sync": "2024-01-01"}"#.to_string()),
                last_sync_at: Some(chrono_now()),
            };
            db.upsert_sync_state(&state).unwrap();

            // 读取并验证
            let states = db.get_sync_states().unwrap();
            assert_eq!(states.len(), 1);
            assert_eq!(states[0].device_id, "device-1");

            // 更新同步状态
            let updated_state = NewSyncState {
                id: "sync-1".to_string(),
                device_id: "device-1".to_string(),
                scope: "incremental".to_string(),
                checkpoint: Some("checkpoint-xyz".to_string()),
                state_json: Some(r#"{"last_sync": "2024-01-02"}"#.to_string()),
                last_sync_at: Some(chrono_now()),
            };
            db.upsert_sync_state(&updated_state).unwrap();

            // 验证更新
            let state = db.get_sync_state("sync-1").unwrap();
            assert_eq!(state.scope, "incremental");

            // 删除同步状态
            db.delete_sync_state("sync-1").unwrap();
            let states = db.get_sync_states().unwrap();
            assert_eq!(states.len(), 0);
        }

        // 清理
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_host_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_host_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create group first (for foreign key)
            let group = NewGroup {
                id: "grp-1".to_string(),
                name: "Test Group".to_string(),
            };
            db.add_group(&group).unwrap();

            // Create host
            let host = NewHost {
                id: "host-1".to_string(),
                name: "Production Server".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "key".to_string(),
                identity_file: Some("/path/to/key".to_string()),
                identity_id: Some("id-1".to_string()),
                group_id: Some("grp-1".to_string()),
                notes: Some("Production server".to_string()),
                color: Some("#FF0000".to_string()),
                environment: Some("production".to_string()),
                region: Some("us-east".to_string()),
                purpose: Some("web".to_string()),
                status: "online".to_string(),
            };
            db.add_host(&host).unwrap();

            // Read and verify
            let hosts = db.get_hosts().unwrap();
            assert_eq!(hosts.len(), 1);
            assert_eq!(hosts[0].name, "Production Server");
            assert_eq!(hosts[0].host, "192.168.1.100");

            // Get single host
            let host = db.get_host("host-1").unwrap();
            assert_eq!(host.name, "Production Server");

            // Update host
            let update = UpdateHost {
                id: "host-1".to_string(),
                name: "Updated Server".to_string(),
                host: "192.168.1.200".to_string(),
                port: 2222,
                username: "root".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: Some("Updated notes".to_string()),
                color: Some("#00FF00".to_string()),
                environment: Some("staging".to_string()),
                region: Some("eu-west".to_string()),
                purpose: Some("database".to_string()),
                status: "offline".to_string(),
            };
            db.update_host(&update).unwrap();

            // Verify update
            let host = db.get_host("host-1").unwrap();
            assert_eq!(host.name, "Updated Server");
            assert_eq!(host.port, 2222);
            assert_eq!(host.status, "offline");

            // Delete host
            db.delete_host("host-1").unwrap();
            let hosts = db.get_hosts().unwrap();
            assert_eq!(hosts.len(), 0);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_identity_crud_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_identity_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create identity
            let identity = NewIdentity {
                id: "id-1".to_string(),
                name: "My Key".to_string(),
                private_key_path: Some("/home/user/.ssh/id_rsa".to_string()),
                passphrase_secret_id: Some("secret-1".to_string()),
                auth_type: "key".to_string(),
            };
            db.add_identity(&identity).unwrap();

            // Read and verify
            let identities = db.get_identities().unwrap();
            assert_eq!(identities.len(), 1);
            assert_eq!(identities[0].name, "My Key");
            assert_eq!(identities[0].auth_type, "key");

            // Get single identity
            let identity = db.get_identity("id-1").unwrap();
            assert_eq!(identity.name, "My Key");

            // Update identity
            let update = UpdateIdentity {
                id: "id-1".to_string(),
                name: "Updated Key".to_string(),
                private_key_path: Some("/home/user/.ssh/id_ed25519".to_string()),
                passphrase_secret_id: Some("secret-2".to_string()),
                auth_type: "agent".to_string(),
            };
            db.update_identity(&update).unwrap();

            // Verify update
            let identity = db.get_identity("id-1").unwrap();
            assert_eq!(identity.name, "Updated Key");
            assert_eq!(identity.auth_type, "agent");

            // Delete identity
            db.delete_identity("id-1").unwrap();
            let identities = db.get_identities().unwrap();
            assert_eq!(identities.len(), 0);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_host_tags_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_host_tags_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create host
            let host = NewHost {
                id: "host-1".to_string(),
                name: "Test Host".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "online".to_string(),
            };
            db.add_host(&host).unwrap();

            // Create tags
            let tag1 = NewTag {
                id: "tag-1".to_string(),
                name: "Production".to_string(),
                color: Some("#FF0000".to_string()),
                description: Some("Production servers".to_string()),
            };
            let tag2 = NewTag {
                id: "tag-2".to_string(),
                name: "Development".to_string(),
                color: Some("#00FF00".to_string()),
                description: Some("Dev servers".to_string()),
            };
            db.add_tag(&tag1).unwrap();
            db.add_tag(&tag2).unwrap();

            // Set host tags
            db.set_host_tag("host-1", "tag-1").unwrap();
            db.set_host_tag("host-1", "tag-2").unwrap();

            // Get host tags
            let host_tags = db.get_host_tags("host-1").unwrap();
            assert_eq!(host_tags.len(), 2);
            assert!(host_tags.iter().any(|t| t.name == "Production"));
            assert!(host_tags.iter().any(|t| t.name == "Development"));

            // Remove host tag
            db.remove_host_tag("host-1", "tag-1").unwrap();
            let host_tags = db.get_host_tags("host-1").unwrap();
            assert_eq!(host_tags.len(), 1);
            assert_eq!(host_tags[0].name, "Development");
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remote_desktop_connection_crud() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_rdp_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create host first
            let host = NewHost {
                id: "host-1".to_string(),
                name: "RDP Server".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "online".to_string(),
            };
            db.add_host(&host).unwrap();

            // Create RDP connection
            let conn = NewRemoteDesktopConnection {
                id: "rdp-1".to_string(),
                host_id: "host-1".to_string(),
                name: "Windows Server".to_string(),
                protocol: "rdp".to_string(),
                host: "192.168.1.101".to_string(),
                port: 3389,
                username: "administrator".to_string(),
                domain: Some("DOMAIN".to_string()),
                password_encrypted: None,
                use_ssh_tunnel: false,
                ssh_host: None,
                ssh_port: None,
                ssh_username: None,
                ssh_auth_type: None,
                display_settings_json: Some(r#"{"resolution": "1920x1080"}"#.to_string()),
                performance_settings_json: None,
                local_resources_json: None,
                experience_settings_json: None,
                gateway_settings_json: None,
                recording_settings_json: None,
                group_id: None,
            };
            db.add_remote_desktop_connection(&conn).unwrap();

            // Read and verify
            let conns = db.get_remote_desktop_connections().unwrap();
            assert_eq!(conns.len(), 1);
            assert_eq!(conns[0].name, "Windows Server");
            assert_eq!(conns[0].protocol, "rdp");

            // Get single connection
            let conn = db.get_remote_desktop_connection("rdp-1").unwrap();
            assert_eq!(conn.host, "192.168.1.101");
            assert!(conn.display_settings_json.is_some());

            // Update connection
            let update = UpdateRemoteDesktopConnection {
                id: "rdp-1".to_string(),
                host_id: "host-1".to_string(),
                name: "Updated RDP".to_string(),
                protocol: "rdp".to_string(),
                host: "192.168.1.102".to_string(),
                port: 3389,
                username: "admin".to_string(),
                domain: None,
                password_encrypted: None,
                use_ssh_tunnel: true,
                ssh_host: Some("192.168.1.100".to_string()),
                ssh_port: Some(22),
                ssh_username: Some("tunnel".to_string()),
                ssh_auth_type: Some("key".to_string()),
                display_settings_json: None,
                performance_settings_json: Some(r#"{"quality": "high"}"#.to_string()),
                local_resources_json: None,
                experience_settings_json: None,
                gateway_settings_json: None,
                recording_settings_json: None,
                group_id: None, // No group to avoid FK constraint
            };
            db.update_remote_desktop_connection(&update).unwrap();

            // Verify update
            let conn = db.get_remote_desktop_connection("rdp-1").unwrap();
            assert_eq!(conn.name, "Updated RDP");
            assert!(conn.use_ssh_tunnel);

            // Delete connection
            db.delete_remote_desktop_connection("rdp-1").unwrap();
            let conns = db.get_remote_desktop_connections().unwrap();
            assert_eq!(conns.len(), 0);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remote_desktop_session_crud() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_rdp_session_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create host
            let host = NewHost {
                id: "host-1".to_string(),
                name: "RDP Server".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                identity_id: None,
                group_id: None,
                notes: None,
                color: None,
                environment: None,
                region: None,
                purpose: None,
                status: "online".to_string(),
            };
            db.add_host(&host).unwrap();

            // Create RDP connection
            let conn = NewRemoteDesktopConnection {
                id: "rdp-1".to_string(),
                host_id: "host-1".to_string(),
                name: "Windows Server".to_string(),
                protocol: "rdp".to_string(),
                host: "192.168.1.101".to_string(),
                port: 3389,
                username: "admin".to_string(),
                domain: None,
                password_encrypted: None,
                use_ssh_tunnel: false,
                ssh_host: None,
                ssh_port: None,
                ssh_username: None,
                ssh_auth_type: None,
                display_settings_json: None,
                performance_settings_json: None,
                local_resources_json: None,
                experience_settings_json: None,
                gateway_settings_json: None,
                recording_settings_json: None,
                group_id: None,
            };
            db.add_remote_desktop_connection(&conn).unwrap();

            // Create session
            let session = NewRemoteDesktopSession {
                id: "rdp-sess-1".to_string(),
                connection_id: "rdp-1".to_string(),
                status: "connecting".to_string(),
                started_at: chrono_now(),
                ended_at: None,
                recording_path: Some("/recordings/session.mp4".to_string()),
                recording_active: true,
            };
            db.add_remote_desktop_session(&session).unwrap();

            // Read and verify
            let sessions = db.get_remote_desktop_sessions().unwrap();
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].connection_id, "rdp-1");
            assert!(sessions[0].recording_active);

            // Get single session
            let session = db.get_remote_desktop_session("rdp-sess-1").unwrap();
            assert_eq!(session.status, "connecting");

            // Update session
            let update = UpdateRemoteDesktopSession {
                id: "rdp-sess-1".to_string(),
                connection_id: "rdp-1".to_string(),
                status: "connected".to_string(),
                started_at: chrono_now(),
                ended_at: Some(chrono_now()),
                recording_path: None,
                recording_active: false,
            };
            db.update_remote_desktop_session(&update).unwrap();

            // Verify update
            let session = db.get_remote_desktop_session("rdp-sess-1").unwrap();
            assert_eq!(session.status, "connected");
            assert!(!session.recording_active);

            // Delete session
            db.delete_remote_desktop_session("rdp-sess-1").unwrap();
            let sessions = db.get_remote_desktop_sessions().unwrap();
            assert_eq!(sessions.len(), 0);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_record_serialization() {
        // Test all record types can be serialized
        let server_record = ServerRecord {
            id: "srv-1".to_string(),
            name: "Test".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "key".to_string(),
            identity_file: Some("/path".to_string()),
            group_id: Some("grp-1".to_string()),
            status: "online".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&server_record).unwrap();
        assert!(json.contains("Test"));

        let host_record = HostRecord {
            id: "host-1".to_string(),
            name: "Test Host".to_string(),
            host: "192.168.1.1".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_type: "password".to_string(),
            identity_file: None,
            identity_id: None,
            group_id: None,
            notes: Some("Notes".to_string()),
            color: Some("#FF0000".to_string()),
            environment: Some("prod".to_string()),
            region: Some("us-east".to_string()),
            purpose: Some("web".to_string()),
            status: "online".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&host_record).unwrap();
        assert!(json.contains("Test Host"));

        let tag_record = TagRecord {
            id: "tag-1".to_string(),
            name: "Production".to_string(),
            color: Some("#FF0000".to_string()),
            description: Some("Prod servers".to_string()),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&tag_record).unwrap();
        assert!(json.contains("Production"));

        let identity_record = IdentityRecord {
            id: "id-1".to_string(),
            name: "My Key".to_string(),
            private_key_path: Some("/path".to_string()),
            passphrase_secret_id: None,
            auth_type: "key".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&identity_record).unwrap();
        assert!(json.contains("My Key"));

        let snippet_record = SnippetRecord {
            id: "snip-1".to_string(),
            name: "List".to_string(),
            command: "ls -la".to_string(),
            description: Some("List files".to_string()),
            folder_id: Some("folder-1".to_string()),
            variables_json: Some("[]".to_string()),
            scope: "personal".to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&snippet_record).unwrap();
        assert!(json.contains("List"));

        let session_record = SessionRecord {
            id: "sess-1".to_string(),
            host_id: "host-1".to_string(),
            title: Some("Session".to_string()),
            status: "active".to_string(),
            last_command: Some("ls".to_string()),
            started_at: chrono_now(),
            ended_at: None,
        };
        let json = serde_json::to_string(&session_record).unwrap();
        assert!(json.contains("Session"));

        let layout_record = LayoutRecord {
            id: "layout-1".to_string(),
            name: "Default".to_string(),
            workspace_mode: "standard".to_string(),
            layout_json: r#"{}"#.to_string(),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&layout_record).unwrap();
        assert!(json.contains("Default"));

        let audit_record = AuditEventRecord {
            id: "audit-1".to_string(),
            actor: Some("user-1".to_string()),
            action: "login".to_string(),
            target_type: Some("server".to_string()),
            target_id: Some("srv-1".to_string()),
            payload_json: Some(r#"{}"#.to_string()),
            level: "info".to_string(),
            created_at: chrono_now(),
        };
        let json = serde_json::to_string(&audit_record).unwrap();
        assert!(json.contains("login"));

        let sync_record = SyncStateRecord {
            id: "sync-1".to_string(),
            device_id: "device-1".to_string(),
            scope: "full".to_string(),
            checkpoint: Some("abc123".to_string()),
            state_json: Some(r#"{}"#.to_string()),
            last_sync_at: Some(chrono_now()),
            created_at: chrono_now(),
            updated_at: chrono_now(),
        };
        let json = serde_json::to_string(&sync_record).unwrap();
        assert!(json.contains("device-1"));
    }

    #[test]
    fn test_database_error_handling() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_error_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Try to get non-existent server
            let result = db.get_server("non-existent");
            assert!(result.is_err());

            // Try to get non-existent host
            let result = db.get_host("non-existent");
            assert!(result.is_err());

            // Try to get non-existent group
            let result = db.get_groups();
            assert!(result.is_ok()); // Empty vec is ok
            assert!(result.unwrap().is_empty());

            // Try to get non-existent tag
            let result = db.get_tag("non-existent");
            assert!(result.is_err());

            // Try to get non-existent snippet
            let result = db.get_snippet("non-existent");
            assert!(result.is_err());

            // Try to get non-existent identity
            let result = db.get_identity("non-existent");
            assert!(result.is_err());

            // Try to get non-existent session
            let result = db.get_session("non-existent");
            assert!(result.is_err());

            // Try to get non-existent layout
            let result = db.get_layout("non-existent");
            assert!(result.is_err());

            // Try to get non-existent sync state
            let result = db.get_sync_state("non-existent");
            assert!(result.is_err());

            // Try to get non-existent RDP connection
            let result = db.get_remote_desktop_connection("non-existent");
            assert!(result.is_err());

            // Try to get non-existent RDP session
            let result = db.get_remote_desktop_session("non-existent");
            assert!(result.is_err());
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_database_multiple_operations() {
        let temp_dir = std::env::temp_dir().join(format!(
            "easyssh_test_multi_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::new(db_path).unwrap();
            db.init().unwrap();

            // Create multiple groups
            for i in 0..10 {
                let group = NewGroup {
                    id: format!("grp-{}", i),
                    name: format!("Group {}", i),
                };
                db.add_group(&group).unwrap();
            }

            let groups = db.get_groups().unwrap();
            assert_eq!(groups.len(), 10);

            // Create multiple servers
            for i in 0..5 {
                let server = NewServer {
                    id: format!("srv-{}", i),
                    name: format!("Server {}", i),
                    host: format!("192.168.1.{}", i),
                    port: 22,
                    username: "admin".to_string(),
                    auth_type: "password".to_string(),
                    identity_file: None,
                    group_id: Some(format!("grp-{}", i % 3)),
                    status: "online".to_string(),
                };
                db.add_server(&server).unwrap();
            }

            let servers = db.get_servers().unwrap();
            assert_eq!(servers.len(), 5);

            // Delete all
            for server in &servers {
                db.delete_server(&server.id).unwrap();
            }

            let servers = db.get_servers().unwrap();
            assert_eq!(servers.len(), 0);

            for group in &groups {
                db.delete_group(&group.id).unwrap();
            }

            let groups = db.get_groups().unwrap();
            assert_eq!(groups.len(), 0);
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
