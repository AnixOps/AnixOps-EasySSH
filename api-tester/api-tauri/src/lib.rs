pub mod commands;

pub use commands::*;

use tauri::Manager;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("api-tester")
        .setup(|app, api| {
            let state = commands::AppState::new()
                .map_err(|e| e.to_string())?;
            app.manage(state);

            let ws_state = commands::WebSocketState::new();
            app.manage(ws_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // HTTP
            commands::execute_request,
            commands::send_http_request,

            // Collections
            commands::save_collection,
            commands::get_collection,
            commands::list_collections,
            commands::delete_collection,
            commands::save_request,
            commands::get_request,
            commands::delete_request,
            commands::duplicate_request,
            commands::search_requests,

            // Environments
            commands::save_environment,
            commands::get_environment,
            commands::list_environments,
            commands::set_active_environment,
            commands::get_active_environment,
            commands::delete_environment,

            // History
            commands::get_history,
            commands::search_history,
            commands::clear_history,
            commands::replay_request,

            // Import/Export
            commands::import_postman_collection,
            commands::import_postman_environment,
            commands::import_curl_command,
            commands::export_postman_collection,
            commands::export_postman_environment,
            commands::export_curl_command,

            // Tests
            commands::run_tests,
            commands::generate_test_script,

            // WebSocket
            commands::ws_connect,
            commands::ws_send,
            commands::ws_get_messages,
            commands::ws_disconnect,
            commands::ws_is_connected,
        ])
        .build()
}
