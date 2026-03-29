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

    pub fn init(&self) -> Result<(), LiteError> {
        self.conn.execute_batch(
            r#"
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

            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
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
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

#[derive(serde::Serialize)]
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
}
