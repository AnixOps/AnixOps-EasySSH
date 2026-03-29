#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose::STANDARD, Engine};
use easyssh_lib::{
    add_group as lib_add_group, add_server as lib_add_server, ai_programming,
    connect_server as lib_connect_server,
    db::{GroupRecord, NewGroup, NewServer, ServerRecord, UpdateGroup, UpdateServer},
    delete_group as lib_delete_group, delete_server as lib_delete_server,
    error::LiteError,
    get_db_path, get_groups as lib_get_groups, get_server as lib_get_server,
    get_servers as lib_get_servers, init_database as lib_init_database,
    ssh_connect as lib_ssh_connect, ssh_disconnect as lib_ssh_disconnect,
    ssh_execute as lib_ssh_execute, ssh_list_sessions as lib_ssh_list_sessions,
    ssh_get_mux_stats as lib_ssh_get_mux_stats,
    update_group as lib_update_group, update_server as lib_update_server, AppState,
};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: std::sync::Mutex::new(None),
            ssh_manager: tokio::sync::Mutex::new(easyssh_lib::ssh::SshSessionManager::new()),
            sftp_manager: tokio::sync::Mutex::new(easyssh_lib::sftp::SftpSessionManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            init_database,
            get_servers,
            get_server,
            add_server,
            update_server,
            delete_server,
            get_groups,
            add_group,
            update_group,
            delete_group,
            connect_server,
            ssh_connect,
            ssh_execute,
            ssh_disconnect,
            ssh_list_sessions,
            ssh_get_mux_stats,
            store_server_password,
            get_server_password,
            delete_server_password,
            init_master_password,
            unlock,
            lock,
            is_unlocked,
            // AI编程接口
            ai_health_check,
            ai_read_code,
            ai_list_files,
            ai_search_code,
            ai_check_rust,
            ai_run_tests,
            ai_build,
            // 底层直接测试接口
            debug_quick_check,
            debug_test_all,
            debug_test_db,
            debug_test_crypto,
            debug_test_ssh,
            debug_test_pro,
            debug_test_terminal,
            // Git操作接口
            git_status,
            git_diff,
            git_log,
            git_branch,
            // 代码修改接口
            write_file,
            edit_file,
            // 上下文管理接口
            set_context,
            get_context,
            clear_context,
        ])
        .run(tauri::generate_context!())
        .expect("启动EasySSH Lite失败");
}

// ==================== Tauri Commands ====================

#[tauri::command]
fn init_database(state: tauri::State<AppState>) -> Result<(), LiteError> {
    lib_init_database(&state)
}

#[tauri::command]
fn get_servers(state: tauri::State<AppState>) -> Result<Vec<ServerRecord>, LiteError> {
    lib_get_servers(&state)
}

#[tauri::command]
fn get_server(state: tauri::State<AppState>, id: String) -> Result<ServerRecord, LiteError> {
    lib_get_server(&state, &id)
}

#[tauri::command]
fn add_server(state: tauri::State<AppState>, server: NewServer) -> Result<(), LiteError> {
    lib_add_server(&state, &server)
}

#[tauri::command]
fn update_server(state: tauri::State<AppState>, server: UpdateServer) -> Result<(), LiteError> {
    lib_update_server(&state, &server)
}

#[tauri::command]
fn delete_server(state: tauri::State<AppState>, id: String) -> Result<(), LiteError> {
    lib_delete_server(&state, &id)
}

#[tauri::command]
fn get_groups(state: tauri::State<AppState>) -> Result<Vec<GroupRecord>, LiteError> {
    lib_get_groups(&state)
}

#[tauri::command]
fn add_group(state: tauri::State<AppState>, group: NewGroup) -> Result<(), LiteError> {
    lib_add_group(&state, &group)
}

#[tauri::command]
fn update_group(state: tauri::State<AppState>, group: UpdateGroup) -> Result<(), LiteError> {
    lib_update_group(&state, &group)
}

#[tauri::command]
fn delete_group(state: tauri::State<AppState>, id: String) -> Result<(), LiteError> {
    lib_delete_group(&state, &id)
}

#[tauri::command]
fn connect_server(state: tauri::State<AppState>, id: String) -> Result<(), LiteError> {
    lib_connect_server(&state, &id)
}

// ==================== SSH嵌入式终端 (Standard) ====================

#[tauri::command]
async fn ssh_connect(
    state: tauri::State<'_, AppState>,
    id: String,
    password: Option<String>,
) -> Result<String, LiteError> {
    lib_ssh_connect(&state, &id, password.as_deref()).await
}

#[tauri::command]
async fn ssh_execute(
    state: tauri::State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<String, LiteError> {
    lib_ssh_execute(&state, &session_id, &command).await
}

#[tauri::command]
async fn ssh_disconnect(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), LiteError> {
    lib_ssh_disconnect(&state, &session_id).await
}

#[tauri::command]
fn ssh_list_sessions(state: tauri::State<AppState>) -> Vec<String> {
    lib_ssh_list_sessions(&state)
}

#[tauri::command]
fn ssh_get_mux_stats(state: tauri::State<AppState>) -> easyssh_lib::ssh::MuxStats {
    lib_ssh_get_mux_stats(&state)
}

// ==================== Keychain ====================

#[tauri::command]
fn store_server_password(server_id: String, password: String) -> Result<(), LiteError> {
    easyssh_lib::keychain::store_password(&server_id, &password)
}

#[tauri::command]
fn get_server_password(server_id: String) -> Result<Option<String>, LiteError> {
    easyssh_lib::keychain::get_password(&server_id)
}

#[tauri::command]
fn delete_server_password(server_id: String) -> Result<(), LiteError> {
    easyssh_lib::keychain::delete_password(&server_id)
}

// ==================== 加密 ====================

#[tauri::command]
fn init_master_password(password: String) -> Result<Vec<u8>, LiteError> {
    let mut crypto = easyssh_lib::crypto::CRYPTO_STATE.lock().unwrap();
    crypto.initialize(&password)?;

    let salt = crypto.get_salt().ok_or(LiteError::InvalidMasterPassword)?;

    let db_path = get_db_path();
    let db = easyssh_lib::db::Database::new(db_path)?;
    db.init()?;
    db.set_config("crypto_salt", &STANDARD.encode(&salt))?;

    Ok(salt)
}

#[tauri::command]
fn unlock(password: String) -> Result<bool, LiteError> {
    let db_path = get_db_path();
    let db = easyssh_lib::db::Database::new(db_path)?;
    db.init()?;

    let salt_base64 = db
        .get_config("crypto_salt")?
        .ok_or(LiteError::InvalidMasterPassword)?;

    let salt = STANDARD
        .decode(&salt_base64)
        .map_err(|_| LiteError::InvalidMasterPassword)?;
    let salt_arr: [u8; 32] = salt
        .try_into()
        .map_err(|_| LiteError::InvalidMasterPassword)?;

    let mut crypto = easyssh_lib::crypto::CRYPTO_STATE.lock().unwrap();
    crypto.set_salt(salt_arr);

    crypto.unlock(&password)
}

#[tauri::command]
fn lock() -> Result<(), LiteError> {
    let mut crypto = easyssh_lib::crypto::CRYPTO_STATE.lock().unwrap();
    crypto.lock();
    Ok(())
}

#[tauri::command]
fn is_unlocked() -> bool {
    let crypto = easyssh_lib::crypto::CRYPTO_STATE.lock().unwrap();
    crypto.is_unlocked()
}

// ==================== AI Programming Interface (Debug) ====================

#[tauri::command]
fn ai_health_check() -> Result<ai_programming::HealthStatus, String> {
    ai_programming::ai_health_check()
}

#[tauri::command]
async fn ai_read_code(path: String) -> Result<String, String> {
    ai_programming::ai_read_code(path).await
}

#[tauri::command]
async fn ai_list_files(dir: String, pattern: Option<String>) -> Result<Vec<String>, String> {
    ai_programming::ai_list_files(dir, pattern).await
}

#[tauri::command]
async fn ai_search_code(
    query: String,
    path: Option<String>,
) -> Result<Vec<ai_programming::SearchResult>, String> {
    ai_programming::ai_search_code(query, path).await
}

#[tauri::command]
async fn ai_check_rust() -> Result<ai_programming::CheckResult, String> {
    ai_programming::ai_check_rust().await
}

#[tauri::command]
async fn ai_run_tests() -> Result<ai_programming::TestResult, String> {
    ai_programming::ai_run_tests().await
}

#[tauri::command]
async fn ai_build() -> Result<ai_programming::BuildResult, String> {
    ai_programming::ai_build().await
}

// ==================== 底层直接测试接口 ====================

#[tauri::command]
fn debug_quick_check() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_quick_check()
}

#[tauri::command]
fn debug_test_all() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_all()
}

#[tauri::command]
fn debug_test_db() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_db()
}

#[tauri::command]
fn debug_test_crypto() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_crypto()
}

#[tauri::command]
fn debug_test_ssh() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_ssh()
}

#[tauri::command]
fn debug_test_pro() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_pro()
}

#[tauri::command]
fn debug_test_terminal() -> Result<ai_programming::DebugTestReport, String> {
    ai_programming::debug_test_terminal()
}

// ==================== Git操作接口 ====================

#[tauri::command]
async fn git_status() -> Result<ai_programming::GitStatus, String> {
    ai_programming::git_status().await
}

#[tauri::command]
async fn git_diff(path: Option<String>) -> Result<String, String> {
    ai_programming::git_diff(path).await
}

#[tauri::command]
async fn git_log(count: usize) -> Result<Vec<ai_programming::GitCommit>, String> {
    ai_programming::git_log(count).await
}

#[tauri::command]
async fn git_branch() -> Result<Vec<ai_programming::GitBranch>, String> {
    ai_programming::git_branch().await
}

// ==================== 代码修改接口 ====================

#[tauri::command]
async fn write_file(path: String, content: String) -> Result<(), String> {
    ai_programming::write_file(path, content).await
}

#[tauri::command]
async fn edit_file(
    path: String,
    old_string: String,
    new_string: String,
) -> Result<ai_programming::EditResult, String> {
    ai_programming::edit_file(path, old_string, new_string).await
}

// ==================== 上下文管理接口 ====================

#[tauri::command]
fn set_context(key: String, value: String) -> Result<(), String> {
    ai_programming::set_context(key, value)
}

#[tauri::command]
fn get_context(key: String) -> Result<Option<String>, String> {
    ai_programming::get_context(key)
}

#[tauri::command]
fn clear_context() -> Result<(), String> {
    ai_programming::clear_context()
}
