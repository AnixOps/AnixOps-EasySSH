use crate::types::*;
use chrono::DateTime;
use dirs::data_dir;
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> ApiResult<Self> {
        let db_path = Self::get_db_path()?;
        let conn = Connection::open(&db_path).map_err(|e| ApiError::Database(e.to_string()))?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    pub fn new_in_memory() -> ApiResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| ApiError::Database(e.to_string()))?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn get_db_path() -> ApiResult<PathBuf> {
        let mut path = data_dir()
            .ok_or_else(|| ApiError::Database("Could not find data directory".to_string()))?;
        path.push("EasySSH");
        path.push("api-tester.db");
        Ok(path)
    }

    fn init_tables(&self) -> ApiResult<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                auth TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS collection_folders (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                parent_id TEXT,
                name TEXT NOT NULL,
                description TEXT,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                headers TEXT NOT NULL,
                query_params TEXT NOT NULL,
                auth TEXT,
                body TEXT NOT NULL,
                pre_request_script TEXT,
                test_script TEXT,
                settings TEXT NOT NULL,
                collection_id TEXT,
                folder_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
                FOREIGN KEY (folder_id) REFERENCES collection_folders(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS environments (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS environment_variables (
                id TEXT PRIMARY KEY,
                environment_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                description TEXT,
                FOREIGN KEY (environment_id) REFERENCES environments(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                request TEXT NOT NULL,
                response TEXT NOT NULL,
                environment_id TEXT,
                collection_id TEXT,
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS websocket_connections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                headers TEXT,
                messages TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_requests_collection ON requests(collection_id);
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp);
            CREATE INDEX IF NOT EXISTS idx_folders_collection ON collection_folders(collection_id);
            ",
            )
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(())
    }

    // Collection operations
    pub fn save_collection(&self, collection: &Collection) -> ApiResult<()> {
        let auth_json = collection
            .auth
            .as_ref()
            .map(|a| serde_json::to_string(a).ok())
            .flatten();

        self.conn.execute(
            "INSERT OR REPLACE INTO collections (id, name, description, auth, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                collection.id,
                collection.name,
                collection.description,
                auth_json,
                collection.created_at.to_rfc3339(),
                collection.updated_at.to_rfc3339()
            ],
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        // Save requests
        for request in &collection.requests {
            self.save_request(request, Some(&collection.id), None)?;
        }

        // Save folders and their requests
        for folder in &collection.folders {
            self.save_folder(folder, &collection.id, None)?;
        }

        Ok(())
    }

    fn save_folder(
        &self,
        folder: &CollectionFolder,
        collection_id: &str,
        parent_id: Option<&str>,
    ) -> ApiResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO collection_folders (id, collection_id, parent_id, name, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                folder.id,
                collection_id,
                parent_id,
                folder.name,
                folder.description
            ],
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        // Save requests in this folder
        for request in &folder.requests {
            self.save_request(request, Some(collection_id), Some(&folder.id))?;
        }

        // Save sub-folders
        for sub_folder in &folder.folders {
            self.save_folder(sub_folder, collection_id, Some(&folder.id))?;
        }

        Ok(())
    }

    pub fn get_collection(&self, id: &str) -> ApiResult<Option<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, auth, created_at, updated_at FROM collections WHERE id = ?1"
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        let row = stmt.query_row([id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let auth_json: Option<String> = row.get(3)?;
            let auth = auth_json.and_then(|s| serde_json::from_str(&s).ok());
            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&chrono::Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&chrono::Utc);

            Ok(Collection {
                id,
                name,
                description,
                requests: Vec::new(),
                folders: Vec::new(),
                variables: Vec::new(),
                auth,
                created_at,
                updated_at,
            })
        });

        match row {
            Ok(mut collection) => {
                // Load requests
                collection.requests = self.get_requests_by_collection(id, None)?;
                // Load folders
                collection.folders = self.get_folders_by_collection(id, None)?;
                Ok(Some(collection))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ApiError::Database(e.to_string())),
        }
    }

    fn get_folders_by_collection(
        &self,
        collection_id: &str,
        parent_id: Option<&str>,
    ) -> ApiResult<Vec<CollectionFolder>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description FROM collection_folders
             WHERE collection_id = ?1 AND (parent_id IS ?2 OR (parent_id IS NULL AND ?2 IS NULL))",
            )
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let parent_param = parent_id.map(|s| s.to_string());
        let rows = stmt
            .query_map(params![collection_id, parent_param], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let mut folders = Vec::new();
        for row in rows {
            let (id, name, description) = row.map_err(|e| ApiError::Database(e.to_string()))?;
            let requests = self.get_requests_by_collection(collection_id, Some(&id))?;
            let sub_folders = self.get_folders_by_collection(collection_id, Some(&id))?;

            folders.push(CollectionFolder {
                id,
                name,
                description,
                requests,
                folders: sub_folders,
            });
        }

        Ok(folders)
    }

    pub fn list_collections(&self) -> ApiResult<Vec<Collection>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM collections ORDER BY updated_at DESC")
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| ApiError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let mut collections = Vec::new();
        for id in ids {
            if let Some(collection) = self.get_collection(&id)? {
                collections.push(collection);
            }
        }

        Ok(collections)
    }

    pub fn delete_collection(&self, id: &str) -> ApiResult<()> {
        self.conn
            .execute("DELETE FROM collections WHERE id = ?1", [id])
            .map_err(|e| ApiError::Database(e.to_string()))?;
        Ok(())
    }

    // Request operations
    pub fn save_request(
        &self,
        request: &ApiRequest,
        collection_id: Option<&str>,
        folder_id: Option<&str>,
    ) -> ApiResult<()> {
        let headers_json = serde_json::to_string(&request.headers)
            .map_err(|e| ApiError::Database(e.to_string()))?;
        let query_params_json = serde_json::to_string(&request.query_params)
            .map_err(|e| ApiError::Database(e.to_string()))?;
        let body_json =
            serde_json::to_string(&request.body).map_err(|e| ApiError::Database(e.to_string()))?;
        let settings_json = serde_json::to_string(&request.settings)
            .map_err(|e| ApiError::Database(e.to_string()))?;
        let auth_json =
            serde_json::to_string(&request.auth).map_err(|e| ApiError::Database(e.to_string()))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO requests
             (id, name, method, url, headers, query_params, auth, body, pre_request_script, test_script, settings, collection_id, folder_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                request.id,
                request.name,
                serde_json::to_string(&request.method).map_err(|e| ApiError::Database(e.to_string()))?,
                request.url,
                headers_json,
                query_params_json,
                auth_json,
                body_json,
                request.pre_request_script,
                request.test_script,
                settings_json,
                collection_id,
                folder_id,
                request.created_at.to_rfc3339(),
                request.updated_at.to_rfc3339()
            ],
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_request(&self, id: &str) -> ApiResult<Option<ApiRequest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, method, url, headers, query_params, auth, body, pre_request_script, test_script, settings, created_at, updated_at
             FROM requests WHERE id = ?1"
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        let row = stmt.query_row([id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let method_str: String = row.get(2)?;
            let method: HttpMethod = serde_json::from_str(&method_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let url: String = row.get(3)?;

            let headers: Vec<KeyValue> =
                serde_json::from_str(&row.get::<_, String>(4)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            let query_params: Vec<KeyValue> = serde_json::from_str(&row.get::<_, String>(5)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            let auth: Auth = serde_json::from_str(&row.get::<_, String>(6)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let body: Body = serde_json::from_str(&row.get::<_, String>(7)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let pre_request_script: Option<String> = row.get(8)?;
            let test_script: Option<String> = row.get(9)?;
            let settings: RequestSettings = serde_json::from_str(&row.get::<_, String>(10)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            let created_at_str: String = row.get(11)?;
            let updated_at_str: String = row.get(12)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&chrono::Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&chrono::Utc);

            Ok(ApiRequest {
                id,
                name,
                method,
                url,
                headers,
                query_params,
                auth,
                body,
                pre_request_script,
                test_script,
                settings,
                created_at,
                updated_at,
            })
        });

        match row {
            Ok(req) => Ok(Some(req)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ApiError::Database(e.to_string())),
        }
    }

    fn get_requests_by_collection(
        &self,
        collection_id: &str,
        folder_id: Option<&str>,
    ) -> ApiResult<Vec<ApiRequest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM requests WHERE collection_id = ?1 AND (folder_id IS ?2 OR (folder_id IS NULL AND ?2 IS NULL))"
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        let folder_param = folder_id.map(|s| s.to_string());
        let ids: Vec<String> = stmt
            .query_map(params![collection_id, folder_param], |row| row.get(0))
            .map_err(|e| ApiError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let mut requests = Vec::new();
        for id in ids {
            if let Some(req) = self.get_request(&id)? {
                requests.push(req);
            }
        }

        Ok(requests)
    }

    pub fn delete_request(&self, id: &str) -> ApiResult<()> {
        self.conn
            .execute("DELETE FROM requests WHERE id = ?1", [id])
            .map_err(|e| ApiError::Database(e.to_string()))?;
        Ok(())
    }

    // Environment operations
    pub fn save_environment(&self, env: &Environment) -> ApiResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO environments (id, name, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    env.id,
                    env.name,
                    if env.is_default { 1 } else { 0 },
                    env.created_at.to_rfc3339(),
                    env.updated_at.to_rfc3339()
                ],
            )
            .map_err(|e| ApiError::Database(e.to_string()))?;

        // Clear existing variables
        self.conn
            .execute(
                "DELETE FROM environment_variables WHERE environment_id = ?1",
                [&env.id],
            )
            .map_err(|e| ApiError::Database(e.to_string()))?;

        // Save variables
        for (i, var) in env.variables.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO environment_variables (id, environment_id, key, value, enabled, description)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("{}-{}", env.id, i),
                    env.id,
                    var.key,
                    var.value,
                    if var.enabled { 1 } else { 0 },
                    var.description
                ],
            ).map_err(|e| ApiError::Database(e.to_string()))?;
        }

        Ok(())
    }

    pub fn get_environment(&self, id: &str) -> ApiResult<Option<Environment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, is_default, created_at, updated_at FROM environments WHERE id = ?1"
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        let row = stmt.query_row([id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let is_default: i32 = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;
            Ok((id, name, is_default, created_at_str, updated_at_str))
        });

        match row {
            Ok((id, name, is_default, created_at_str, updated_at_str)) => {
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| ApiError::Database(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| ApiError::Database(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                let variables = self.get_environment_variables(&id)?;
                Ok(Some(Environment {
                    id,
                    name,
                    is_default: is_default != 0,
                    variables,
                    created_at,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ApiError::Database(e.to_string())),
        }
    }

    fn get_environment_variables(&self, env_id: &str) -> ApiResult<Vec<EnvironmentVariable>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, value, enabled, description FROM environment_variables WHERE environment_id = ?1"
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        let vars = stmt
            .query_map([env_id], |row| {
                Ok(EnvironmentVariable {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    enabled: row.get::<_, i32>(2)? != 0,
                    description: row.get(3)?,
                })
            })
            .map_err(|e| ApiError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(vars)
    }

    pub fn list_environments(&self) -> ApiResult<Vec<Environment>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM environments ORDER BY is_default DESC, name ASC")
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| ApiError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let mut environments = Vec::new();
        for id in ids {
            if let Some(env) = self.get_environment(&id)? {
                environments.push(env);
            }
        }

        Ok(environments)
    }

    pub fn delete_environment(&self, id: &str) -> ApiResult<()> {
        self.conn
            .execute("DELETE FROM environments WHERE id = ?1", [id])
            .map_err(|e| ApiError::Database(e.to_string()))?;
        Ok(())
    }

    // History operations
    pub fn save_history(&self, entry: &HistoryEntry) -> ApiResult<()> {
        let request_json =
            serde_json::to_string(&entry.request).map_err(|e| ApiError::Database(e.to_string()))?;
        let response_json = serde_json::to_string(&entry.response)
            .map_err(|e| ApiError::Database(e.to_string()))?;

        self.conn.execute(
            "INSERT INTO history (id, request, response, environment_id, collection_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.id,
                request_json,
                response_json,
                entry.environment_id,
                entry.collection_id,
                entry.timestamp.to_rfc3339()
            ],
        ).map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(())
    }

    pub fn get_history(&self, limit: usize) -> ApiResult<Vec<HistoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, request, response, environment_id, collection_id, timestamp
             FROM history ORDER BY timestamp DESC LIMIT ?1",
            )
            .map_err(|e| ApiError::Database(e.to_string()))?;

        let entries = stmt
            .query_map([limit], |row| {
                let id: String = row.get(0)?;
                let request: ApiRequest =
                    serde_json::from_str(&row.get::<_, String>(1)?).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                let response: ApiResponse = serde_json::from_str(&row.get::<_, String>(2)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                let environment_id: Option<String> = row.get(3)?;
                let collection_id: Option<String> = row.get(4)?;
                let timestamp_str: String = row.get(5)?;

                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                Ok(HistoryEntry {
                    id,
                    request,
                    response,
                    environment_id,
                    collection_id,
                    timestamp,
                })
            })
            .map_err(|e| ApiError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Database(e.to_string()))?;

        Ok(entries)
    }

    pub fn clear_history(&self, older_than_days: Option<i64>) -> ApiResult<()> {
        match older_than_days {
            Some(days) => {
                let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
                self.conn
                    .execute(
                        "DELETE FROM history WHERE timestamp < ?1",
                        [cutoff.to_rfc3339()],
                    )
                    .map_err(|e| ApiError::Database(e.to_string()))?;
            }
            None => {
                self.conn
                    .execute("DELETE FROM history", [])
                    .map_err(|e| ApiError::Database(e.to_string()))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_crud() {
        let db = Database::new_in_memory().unwrap();

        // Test collection
        let collection = Collection {
            id: "test-col-1".to_string(),
            name: "Test Collection".to_string(),
            description: None,
            requests: vec![],
            folders: vec![],
            variables: vec![],
            auth: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        db.save_collection(&collection).unwrap();

        let retrieved = db.get_collection("test-col-1").unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Collection");
    }
}
