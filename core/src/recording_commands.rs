//! Tauri commands for session recording
//! Exposes session recording functionality to the frontend

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use crate::error::LiteError;
use crate::session_recording::{
    CloudShareConfig, CloudShareManager, ExportFormat, ExportOptions, RecordingConfig,
    RecordingMetadata, RecordingState, SessionRecordingManager,
};

/// Recording manager state for Tauri
pub struct RecordingStateWrapper {
    manager: Arc<Mutex<SessionRecordingManager>>,
}

impl RecordingStateWrapper {
    pub fn new(storage_path: std::path::PathBuf) -> Result<Self, LiteError> {
        let manager = SessionRecordingManager::new(&storage_path)?;
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }
}

/// Request to start recording
#[derive(Debug, Clone, Deserialize)]
pub struct StartRecordingRequest {
    pub width: u32,
    pub height: u32,
    pub title: Option<String>,
    pub record_input: bool,
    pub enable_privacy_filter: bool,
    pub auto_mark_commands: bool,
    pub server_id: Option<String>,
}

/// Request to add a mark
#[derive(Debug, Clone, Deserialize)]
pub struct AddMarkRequest {
    pub recording_id: String,
    pub label: String,
    pub color: Option<String>,
}

/// Request to export recording
#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    pub recording_id: String,
    pub format: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
}

/// Response for export operation
#[derive(Debug, Clone, Serialize)]
pub struct ExportResponse {
    pub success: bool,
    pub file_path: String,
    pub error: Option<String>,
}

/// Request to upload to asciinema.org
#[derive(Debug, Clone, Deserialize)]
pub struct UploadRequest {
    pub recording_id: String,
    pub title: Option<String>,
}

/// Response for upload operation
#[derive(Debug, Clone, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}

/// Start a new recording session
#[tauri::command]
pub async fn recording_start(
    state: State<'_, RecordingStateWrapper>,
    request: StartRecordingRequest,
) -> Result<String, String> {
    let manager = state.manager.lock().await;

    let config = RecordingConfig {
        width: request.width,
        height: request.height,
        title: request.title,
        record_input: request.record_input,
        enable_privacy_filter: request.enable_privacy_filter,
        auto_mark_commands: request.auto_mark_commands,
        idle_time_limit: Some(1.0),
        max_duration: None,
        output_dir: crate::get_db_path()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("recordings"),
        command: None,
    };

    match manager.start_recording(config, request.server_id).await {
        Ok(recording_id) => Ok(recording_id),
        Err(e) => Err(e.to_string()),
    }
}

/// Record output data to a recording
#[tauri::command]
pub async fn recording_record_output(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
    data: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.record_output(&recording_id, &data).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Record input data to a recording
#[tauri::command]
pub async fn recording_record_input(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
    data: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.record_input(&recording_id, &data).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Record terminal resize
#[tauri::command]
pub async fn recording_record_resize(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.record_resize(&recording_id, width, height).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Add a mark to a recording
#[tauri::command]
pub async fn recording_add_mark(
    state: State<'_, RecordingStateWrapper>,
    request: AddMarkRequest,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager
        .add_mark(
            &request.recording_id,
            &request.label,
            request.color.as_deref(),
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Pause a recording
#[tauri::command]
pub async fn recording_pause(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.pause_recording(&recording_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Resume a recording
#[tauri::command]
pub async fn recording_resume(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.resume_recording(&recording_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Stop a recording
#[tauri::command]
pub async fn recording_stop(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<RecordingMetadata, String> {
    let manager = state.manager.lock().await;

    match manager.stop_recording(&recording_id).await {
        Ok(metadata) => Ok(metadata),
        Err(e) => Err(e.to_string()),
    }
}

/// Get recording state
#[tauri::command]
pub async fn recording_get_state(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<Option<RecordingState>, String> {
    let manager = state.manager.lock().await;

    match manager.get_recording_state(&recording_id).await {
        Some(state) => Ok(Some(state)),
        None => Ok(None),
    }
}

/// List all recordings
#[tauri::command]
pub async fn recording_list(
    state: State<'_, RecordingStateWrapper>,
) -> Result<Vec<RecordingMetadata>, String> {
    let manager = state.manager.lock().await;
    Ok(manager.list_recordings().await)
}

/// Delete a recording
#[tauri::command]
pub async fn recording_delete(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<(), String> {
    let manager = state.manager.lock().await;

    match manager.delete_recording(&recording_id).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Search in recordings
#[tauri::command]
pub async fn recording_search(
    state: State<'_, RecordingStateWrapper>,
    query: String,
) -> Result<Vec<(String, String, f64)>, String> {
    let manager = state.manager.lock().await;

    // Search across all recordings and flatten results
    let results = manager.search_all_recordings(&query).await;

    // Convert to serializable format: (recording_id, preview, timestamp)
    let mut flattened = Vec::new();
    for (recording_id, matches) in results {
        for m in matches {
            flattened.push((recording_id.clone(), m.preview.clone(), m.timestamp));
        }
    }

    Ok(flattened)
}

/// Export a recording
#[tauri::command]
pub async fn recording_export(
    state: State<'_, RecordingStateWrapper>,
    request: ExportRequest,
    app: AppHandle,
) -> Result<ExportResponse, String> {
    let manager = state.manager.lock().await;

    // Get recording path
    let recordings = manager.list_recordings().await;
    let recording = recordings
        .iter()
        .find(|r| r.id == request.recording_id)
        .ok_or("Recording not found")?;

    // Determine export format
    let format = match request.format.as_str() {
        "asciicast" => ExportFormat::Asciicast,
        "json" => ExportFormat::Json,
        "text" => ExportFormat::Text,
        "gif" => ExportFormat::Gif,
        "mp4" => ExportFormat::Mp4,
        _ => return Err("Invalid format".to_string()),
    };

    // Get app data directory
    let app_data = app
        .path_resolver()
        .app_data_dir()
        .ok_or("Failed to get app data dir")?;
    let exports_dir = app_data.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;

    let output_path = exports_dir.join(format!(
        "{}.{}",
        request.recording_id,
        match format {
            ExportFormat::Asciicast => "cast",
            ExportFormat::Json => "json",
            ExportFormat::Text => "txt",
            ExportFormat::Gif => "gif",
            ExportFormat::Mp4 => "mp4",
        }
    ));

    let options = ExportOptions {
        format,
        start_time: request.start_time,
        end_time: request.end_time,
        width: None,
        height: None,
        quality: Some(80),
    };

    let export_manager = crate::session_recording::ExportManager::new();

    match export_manager
        .export(
            std::path::Path::new(&recording.file_path),
            &output_path,
            options,
        )
        .await
    {
        Ok(_) => Ok(ExportResponse {
            success: true,
            file_path: output_path.to_string_lossy().to_string(),
            error: None,
        }),
        Err(e) => Ok(ExportResponse {
            success: false,
            file_path: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

/// Upload recording to asciinema.org
#[tauri::command]
pub async fn recording_upload(
    state: State<'_, RecordingStateWrapper>,
    request: UploadRequest,
) -> Result<UploadResponse, String> {
    let manager = state.manager.lock().await;

    // Get recording
    let recordings = manager.list_recordings().await;
    let recording = recordings
        .iter()
        .find(|r| r.id == request.recording_id)
        .ok_or("Recording not found")?;

    let config = CloudShareConfig::default();
    let share_manager = CloudShareManager::new(config);

    match share_manager
        .upload_to_asciinema(
            std::path::Path::new(&recording.file_path),
            request.title.as_deref(),
        )
        .await
    {
        Ok(url) => Ok(UploadResponse {
            success: true,
            url: Some(url),
            error: None,
        }),
        Err(e) => Ok(UploadResponse {
            success: false,
            url: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Get player data for a recording
#[tauri::command]
pub async fn recording_get_player_data(
    state: State<'_, RecordingStateWrapper>,
    recording_id: String,
) -> Result<String, String> {
    let manager = state.manager.lock().await;

    // Get recording path
    let recordings = manager.list_recordings().await;
    let recording = recordings
        .iter()
        .find(|r| r.id == recording_id)
        .ok_or("Recording not found")?;

    // Read recording content
    match std::fs::read_to_string(&recording.file_path) {
        Ok(content) => Ok(content),
        Err(e) => Err(e.to_string()),
    }
}
