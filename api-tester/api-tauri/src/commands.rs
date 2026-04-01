use api_tester_core::collection::CollectionManager;
use api_tester_core::history::HistoryManager;
use api_tester_core::*;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// Application state
pub struct AppState {
    http_client: Arc<Mutex<HttpClient>>,
    db: Arc<Mutex<Database>>,
    env_manager: Arc<Mutex<EnvironmentManager>>,
    collection_manager: Arc<Mutex<CollectionManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    test_runner: Arc<TestRunner>,
}

impl AppState {
    pub fn new() -> ApiResult<Self> {
        Ok(Self {
            http_client: Arc::new(Mutex::new(HttpClient::new()?)),
            db: Arc::new(Mutex::new(Database::new()?)),
            env_manager: Arc::new(Mutex::new(EnvironmentManager::new())),
            collection_manager: Arc::new(Mutex::new(CollectionManager::new())),
            history_manager: Arc::new(Mutex::new(HistoryManager::new(1000))),
            test_runner: Arc::new(TestRunner::new()),
        })
    }
}

// HTTP Commands
#[tauri::command]
pub async fn execute_request(
    state: State<'_, AppState>,
    mut request: ApiRequest,
    environment_id: Option<String>,
) -> Result<(ApiResponse, Vec<TestResult>), String> {
    // Apply environment variables
    {
        let mut env_manager = state.env_manager.lock().await;
        if let Some(env_id) = &environment_id {
            env_manager.set_active(Some(env_id.clone()));
        }
        env_manager.apply_to_request(&mut request);
    }

    // Execute request
    let client = state.http_client.lock().await;
    let response = client.execute(&request).await.map_err(|e| e.to_string())?;

    // Run tests if test script exists
    let test_results = if let Some(script) = &request.test_script {
        state.test_runner.run_tests(script, &response)
    } else {
        Vec::new()
    };

    // Save to history
    {
        let mut history = state.history_manager.lock().await;
        history.add_entry(HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            request: request.clone(),
            response: response.clone(),
            environment_id,
            collection_id: None,
            timestamp: chrono::Utc::now(),
        });

        // Also save to database
        let db = state.db.lock().await;
        let _ = db.save_history(&HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            response: response.clone(),
            environment_id: None,
            collection_id: None,
            timestamp: chrono::Utc::now(),
        });
    }

    Ok((response, test_results))
}

#[tauri::command]
pub async fn send_http_request(
    state: State<'_, AppState>,
    url: String,
    method: HttpMethod,
    headers: Vec<KeyValue>,
    body: Body,
    auth: Auth,
) -> Result<ApiResponse, String> {
    let request = ApiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Quick Request".to_string(),
        method,
        url,
        headers,
        query_params: Vec::new(),
        auth,
        body,
        pre_request_script: None,
        test_script: None,
        settings: RequestSettings::default(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let client = state.http_client.lock().await;
    client.execute(&request).await.map_err(|e| e.to_string())
}

// Collection Commands
#[tauri::command]
pub async fn save_collection(
    state: State<'_, AppState>,
    collection: Collection,
) -> Result<(), String> {
    // Save to manager
    {
        let mut manager = state.collection_manager.lock().await;
        manager.add_collection(collection.clone());
    }

    // Save to database
    let db = state.db.lock().await;
    db.save_collection(&collection).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_collection(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Collection>, String> {
    // First check manager
    {
        let manager = state.collection_manager.lock().await;
        if let Some(c) = manager.get_collection(&id) {
            return Ok(Some(c.clone()));
        }
    }

    // Fall back to database
    let db = state.db.lock().await;
    db.get_collection(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    let db = state.db.lock().await;
    db.list_collections().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_collection(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Remove from manager
    {
        let mut manager = state.collection_manager.lock().await;
        manager.remove_collection(&id);
    }

    // Remove from database
    let db = state.db.lock().await;
    db.delete_collection(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_request(
    state: State<'_, AppState>,
    request: ApiRequest,
    collection_id: Option<String>,
    folder_id: Option<String>,
) -> Result<(), String> {
    // Save to manager
    {
        let mut manager = state.collection_manager.lock().await;
        if let Some(cid) = &collection_id {
            manager
                .add_request_to_collection(cid, request.clone(), folder_id.as_deref())
                .map_err(|e| e.to_string())?;
        }
    }

    // Save to database
    let db = state.db.lock().await;
    db.save_request(&request, collection_id.as_deref(), folder_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_request(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ApiRequest>, String> {
    // Check manager first
    {
        let manager = state.collection_manager.lock().await;
        if let Some((_, req)) = manager.find_request(&id) {
            return Ok(Some(req.clone()));
        }
    }

    // Fall back to database
    let db = state.db.lock().await;
    db.get_request(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_request(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Remove from manager
    {
        let mut manager = state.collection_manager.lock().await;
        manager.delete_request(&id).map_err(|e| e.to_string())?;
    }

    // Remove from database
    let db = state.db.lock().await;
    db.delete_request(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn duplicate_request(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiRequest, String> {
    let mut manager = state.collection_manager.lock().await;
    manager.duplicate_request(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_requests(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ApiRequest>, String> {
    let manager = state.collection_manager.lock().await;
    Ok(manager.search(&query).into_iter().cloned().collect())
}

// Environment Commands
#[tauri::command]
pub async fn save_environment(state: State<'_, AppState>, env: Environment) -> Result<(), String> {
    // Save to manager
    {
        let mut manager = state.env_manager.lock().await;
        manager.add_environment(env.clone());
    }

    // Save to database
    let db = state.db.lock().await;
    db.save_environment(&env).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_environment(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<Environment>, String> {
    // Check manager first
    {
        let manager = state.env_manager.lock().await;
        if let Some(e) = manager.get_environment(&id) {
            return Ok(Some(e.clone()));
        }
    }

    // Fall back to database
    let db = state.db.lock().await;
    db.get_environment(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_environments(state: State<'_, AppState>) -> Result<Vec<Environment>, String> {
    let db = state.db.lock().await;
    db.list_environments().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_active_environment(
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<(), String> {
    let mut manager = state.env_manager.lock().await;
    manager.set_active(id);
    Ok(())
}

#[tauri::command]
pub async fn get_active_environment(
    state: State<'_, AppState>,
) -> Result<Option<Environment>, String> {
    let manager = state.env_manager.lock().await;
    Ok(manager.get_active().cloned())
}

#[tauri::command]
pub async fn delete_environment(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Remove from manager
    {
        let mut manager = state.env_manager.lock().await;
        manager.remove_environment(&id);
    }

    // Remove from database
    let db = state.db.lock().await;
    db.delete_environment(&id).map_err(|e| e.to_string())
}

// History Commands
#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    limit: usize,
) -> Result<Vec<HistoryEntry>, String> {
    // Check manager first
    {
        let manager = state.history_manager.lock().await;
        let entries: Vec<_> = manager.get_entries(limit).into_iter().cloned().collect();
        if !entries.is_empty() {
            return Ok(entries);
        }
    }

    // Fall back to database
    let db = state.db.lock().await;
    db.get_history(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_history(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<HistoryEntry>, String> {
    let manager = state.history_manager.lock().await;
    Ok(manager.search(&query).into_iter().cloned().collect())
}

#[tauri::command]
pub async fn clear_history(
    state: State<'_, AppState>,
    older_than_days: Option<i64>,
) -> Result<(), String> {
    // Clear manager
    {
        let mut manager = state.history_manager.lock().await;
        if let Some(days) = older_than_days {
            manager.clear_older_than(days);
        } else {
            manager.clear();
        }
    }

    // Clear database
    let db = state.db.lock().await;
    db.clear_history(older_than_days).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn replay_request(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<Option<ApiRequest>, String> {
    let manager = state.history_manager.lock().await;
    Ok(manager.replay_request(&entry_id))
}

// Import/Export Commands
#[tauri::command]
pub fn import_postman_collection(data: String) -> Result<Collection, String> {
    let importer = Importer::new();
    importer
        .import_postman_collection(&data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_postman_environment(data: String) -> Result<Environment, String> {
    let importer = Importer::new();
    importer
        .import_postman_environment(&data)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_curl_command(command: String) -> Result<ApiRequest, String> {
    let importer = Importer::new();
    importer.import_curl(&command).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_postman_collection(collection: Collection) -> Result<String, String> {
    let exporter = Exporter::new();
    exporter
        .export_postman_collection(&collection)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_postman_environment(env: Environment) -> Result<String, String> {
    let exporter = Exporter::new();
    exporter
        .export_postman_environment(&env)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_curl_command(request: ApiRequest) -> Result<String, String> {
    let exporter = Exporter::new();
    exporter.export_curl(&request).map_err(|e| e.to_string())
}

// Test Commands
#[tauri::command]
pub fn run_tests(test_script: String, response: ApiResponse) -> Result<Vec<TestResult>, String> {
    let runner = TestRunner::new();
    Ok(runner.run_tests(&test_script, &response))
}

#[tauri::command]
pub fn generate_test_script(response: ApiResponse) -> Result<String, String> {
    Ok(test_runner::generate_test_script(&response))
}

// WebSocket Commands
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct WebSocketState {
    clients: Arc<RwLock<HashMap<String, WebSocketClient>>>,
}

impl WebSocketState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tauri::command]
pub async fn ws_connect(
    state: State<'_, WebSocketState>,
    id: String,
    url: String,
    headers: Option<Vec<KeyValue>>,
) -> Result<(), String> {
    let mut client = WebSocketClient::new(url);
    client.connect(headers).await.map_err(|e| e.to_string())?;

    let mut clients = state.clients.write().await;
    clients.insert(id, client);

    Ok(())
}

#[tauri::command]
pub async fn ws_send(
    state: State<'_, WebSocketState>,
    id: String,
    message: String,
) -> Result<(), String> {
    let clients = state.clients.read().await;
    let client = clients.get(&id).ok_or("WebSocket not found")?;
    client.send(message).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ws_get_messages(
    state: State<'_, WebSocketState>,
    id: String,
) -> Result<Vec<WebSocketMessage>, String> {
    let clients = state.clients.read().await;
    let client = clients.get(&id).ok_or("WebSocket not found")?;
    Ok(client.get_messages().await)
}

#[tauri::command]
pub async fn ws_disconnect(state: State<'_, WebSocketState>, id: String) -> Result<(), String> {
    let mut clients = state.clients.write().await;
    if let Some(mut client) = clients.remove(&id) {
        client.disconnect().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn ws_is_connected(state: State<'_, WebSocketState>, id: String) -> Result<bool, String> {
    let clients = state.clients.read().await;
    let client = clients.get(&id).ok_or("WebSocket not found")?;
    Ok(client.is_connected().await)
}
