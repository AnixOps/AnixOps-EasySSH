//! UI Integration for Auto-Update System
//!
//! This module provides high-level functions for UI platforms to integrate
//! the auto-update system with minimal code.

use crate::auto_update::{
    AutoUpdater, UpdateConfig, UpdateInfo, UpdateProgress, UpdateResponse, UpdateResult,
    UpdateStage, CURRENT_VERSION,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// UI-friendly update controller
pub struct UpdateController {
    updater: Arc<AutoUpdater>,
    ui_callback: Arc<RwLock<Option<Box<dyn Fn(UpdateUiEvent) + Send + Sync>>>>,
}

#[derive(Debug, Clone)]
pub enum UpdateUiEvent {
    /// No update available
    NoUpdate,
    /// Update available, user needs to choose
    UpdateAvailable {
        info: UpdateInfo,
        is_mandatory: bool,
    },
    /// Download started
    DownloadStarted { version: String },
    /// Download progress
    DownloadProgress {
        percentage: f32,
        bytes_downloaded: u64,
        total_bytes: u64,
        speed: String,
    },
    /// Download completed
    DownloadCompleted { version: String, path: String },
    /// Installation started
    InstallationStarted { version: String },
    /// Installation progress
    InstallationProgress { percentage: f32, message: String },
    /// Installation completed, ready to restart
    InstallationCompleted { version: String },
    /// Error occurred
    Error {
        message: String,
        can_retry: bool,
        can_rollback: bool,
    },
    /// Rollback completed
    RollbackCompleted { previous_version: String },
    /// Update skipped
    UpdateSkipped { version: String },
    /// Remind later set
    RemindLaterSet {
        version: String,
        remind_time: String,
    },
}

impl UpdateController {
    /// Create new update controller with default config
    pub async fn new() -> anyhow::Result<Self> {
        let config = UpdateConfig::default();
        Self::with_config(config).await
    }

    /// Create with custom config
    pub async fn with_config(config: UpdateConfig) -> anyhow::Result<Self> {
        let updater = Arc::new(AutoUpdater::new(config).await?);

        Ok(Self {
            updater,
            ui_callback: Arc::new(RwLock::new(None)),
        })
    }

    /// Set UI event callback
    pub async fn set_ui_callback<F>(&self, callback: F)
    where
        F: Fn(UpdateUiEvent) + Send + Sync + 'static,
    {
        let mut cb = self.ui_callback.write().await;
        *cb = Some(Box::new(callback));
    }

    /// Check for updates (manual trigger)
    pub async fn check_now(&self) -> anyhow::Result<()> {
        let result = self.updater.check_for_updates(false).await?;

        match result {
            UpdateResult::UpdateAvailable(info) => {
                let is_mandatory = self.updater.is_mandatory_update(&info).await;
                self.emit_event(UpdateUiEvent::UpdateAvailable { info, is_mandatory })
                    .await;
            }
            UpdateResult::NoUpdateAvailable => {
                self.emit_event(UpdateUiEvent::NoUpdate).await;
            }
            _ => {}
        }

        Ok(())
    }

    /// Start background update checking
    pub async fn start_background_checks(self: Arc<Self>) {
        let updater = self.updater.clone();

        tokio::spawn(async move {
            let controller = Arc::new(UpdateController {
                updater,
                ui_callback: Arc::new(RwLock::new(None)),
            });

            // Use the AutoUpdater's built-in background checker
            let updater_arc = controller.updater.clone();
            let _ = tokio::spawn(async move {
                updater_arc.start_background_checker().await;
            });
        });
    }

    /// Handle user choosing to install now
    pub async fn install_now(&self, info: UpdateInfo) -> anyhow::Result<()> {
        self.emit_event(UpdateUiEvent::DownloadStarted {
            version: info.version.clone(),
        })
        .await;

        // Subscribe to progress
        let mut progress_rx = self.updater.subscribe_progress().await;

        // Spawn progress monitoring
        let updater = self.updater.clone();
        let info_clone = info.clone();
        let controller = self.clone_without_callback();

        let download_handle = tokio::spawn(async move {
            // Process progress updates
            while let Some(progress) = progress_rx.recv().await {
                controller.handle_progress(&info_clone, &progress).await;
            }
        });

        // Download
        let path = match self.updater.download_update(&info).await {
            Ok(p) => p,
            Err(e) => {
                self.emit_event(UpdateUiEvent::Error {
                    message: format!("Download failed: {}", e),
                    can_retry: true,
                    can_rollback: false,
                })
                .await;
                return Err(e);
            }
        };

        download_handle.abort();

        self.emit_event(UpdateUiEvent::DownloadCompleted {
            version: info.version.clone(),
            path: path.display().to_string(),
        })
        .await;

        // Install
        self.emit_event(UpdateUiEvent::InstallationStarted {
            version: info.version.clone(),
        })
        .await;

        match self.updater.install_update(&info, &path).await {
            Ok(_) => {
                self.emit_event(UpdateUiEvent::InstallationCompleted {
                    version: info.version,
                })
                .await;
                Ok(())
            }
            Err(e) => {
                let can_rollback = self.updater.get_state().await.0 == UpdateStage::RollingBack;
                self.emit_event(UpdateUiEvent::Error {
                    message: format!("Installation failed: {}", e),
                    can_retry: true,
                    can_rollback,
                })
                .await;
                Err(e)
            }
        }
    }

    /// Handle user choosing to install later
    pub async fn install_later(&self, info: UpdateInfo) -> anyhow::Result<()> {
        let result = self
            .updater
            .handle_user_response(super::UpdateResponse::InstallLater, &info)
            .await?;

        if let UpdateResult::RemindLater { version } = result {
            let remind_time = chrono::Local::now() + chrono::Duration::hours(24);
            self.emit_event(UpdateUiEvent::RemindLaterSet {
                version,
                remind_time: remind_time.to_rfc3339(),
            })
            .await;
        }

        Ok(())
    }

    /// Handle user choosing to skip version
    pub async fn skip_version(&self, info: UpdateInfo) -> anyhow::Result<()> {
        let result = self
            .updater
            .handle_user_response(super::UpdateResponse::SkipVersion, &info)
            .await?;

        if let UpdateResult::Skipped { version } = result {
            self.emit_event(UpdateUiEvent::UpdateSkipped { version })
                .await;
        }

        Ok(())
    }

    /// Force rollback to previous version
    pub async fn rollback(&self) -> anyhow::Result<()> {
        match self.updater.rollback().await? {
            UpdateResult::RolledBack { previous_version } => {
                self.emit_event(UpdateUiEvent::RollbackCompleted { previous_version })
                    .await;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Rollback failed")),
        }
    }

    /// Get current status
    pub async fn get_status(&self) -> (UpdateStage, Option<UpdateInfo>) {
        self.updater.get_state().await
    }

    /// Get current version
    pub fn get_current_version(&self) -> &'static str {
        CURRENT_VERSION
    }

    /// Check if rollback is available
    pub async fn can_rollback(&self) -> bool {
        // Access rollback manager through the updater
        // This would need to be exposed in the AutoUpdater
        false // Placeholder
    }

    // Helper methods
    async fn emit_event(&self, event: UpdateUiEvent) {
        let cb = self.ui_callback.read().await;
        if let Some(ref callback) = *cb {
            callback(event);
        }
    }

    async fn handle_progress(&self, _info: &UpdateInfo, progress: &super::UpdateProgress) {
        let speed = format_speed(progress.speed_bps);

        match progress.stage {
            UpdateStage::Downloading => {
                self.emit_event(UpdateUiEvent::DownloadProgress {
                    percentage: progress.percentage,
                    bytes_downloaded: progress.bytes_downloaded,
                    total_bytes: progress.total_bytes,
                    speed,
                })
                .await;
            }
            UpdateStage::Installing => {
                self.emit_event(UpdateUiEvent::InstallationProgress {
                    percentage: progress.percentage,
                    message: progress.message.clone(),
                })
                .await;
            }
            _ => {}
        }
    }

    fn clone_without_callback(&self) -> Self {
        Self {
            updater: self.updater.clone(),
            ui_callback: Arc::new(RwLock::new(None)),
        }
    }
}

fn format_speed(bps: f64) -> String {
    if bps > 1_000_000_000.0 {
        format!("{:.2} GB/s", bps / 1_000_000_000.0)
    } else if bps > 1_000_000.0 {
        format!("{:.2} MB/s", bps / 1_000_000.0)
    } else if bps > 1_000.0 {
        format!("{:.2} KB/s", bps / 1_000.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

/// Predefined configurations for different update strategies
pub mod presets {
    use crate::auto_update::UpdateChannel;
    use crate::auto_update::UpdateConfig;

    /// Aggressive auto-update: auto-download and install
    pub fn aggressive() -> UpdateConfig {
        UpdateConfig {
            auto_download: true,
            auto_install: true,
            silent_mode: true,
            ..Default::default()
        }
    }

    /// Standard auto-update: auto-download, prompt for install
    pub fn standard() -> UpdateConfig {
        UpdateConfig {
            auto_download: true,
            auto_install: false,
            silent_mode: false,
            ..Default::default()
        }
    }

    /// Conservative: prompt for everything
    pub fn conservative() -> UpdateConfig {
        UpdateConfig {
            auto_download: false,
            auto_install: false,
            silent_mode: false,
            ..Default::default()
        }
    }

    /// Beta tester: beta channel, auto-update
    pub fn beta_tester() -> UpdateConfig {
        UpdateConfig {
            channel: UpdateChannel::Beta,
            auto_download: true,
            auto_install: true,
            include_beta: true,
            ..Default::default()
        }
    }

    /// Enterprise: stable, manual updates only
    pub fn enterprise() -> UpdateConfig {
        UpdateConfig {
            channel: UpdateChannel::Stable,
            auto_download: false,
            auto_install: false,
            check_interval: 86400, // 24 hours
            ..Default::default()
        }
    }
}

/// Quick-start function for UI platforms
pub async fn init_auto_update() -> anyhow::Result<UpdateController> {
    let config = presets::standard();
    UpdateController::with_config(config).await
}

#[cfg(feature = "gtk")]
pub mod gtk_integration {
    use super::*;
    use gtk::glib;

    /// Setup auto-update for GTK4 app
    pub async fn setup_gtk_updater() -> anyhow::Result<UpdateController> {
        let controller = init_auto_update().await?;

        controller
            .set_ui_callback(move |event| {
                glib::idle_add_local_once(move || {
                    // Emit GTK signal or update UI
                    handle_gtk_update_event(event);
                });
            })
            .await;

        Ok(controller)
    }

    fn handle_gtk_update_event(_event: UpdateUiEvent) {
        // Implementation would emit GTK signal
    }
}

#[cfg(feature = "windows-ui")]
pub mod windows_integration {
    use super::*;

    /// Setup auto-update for WinUI3 app
    pub async fn setup_winui_updater() -> anyhow::Result<UpdateController> {
        let controller = init_auto_update().await?;

        controller
            .set_ui_callback(move |event| {
                // Send message to WinUI dispatcher
                // Implementation depends on WinRT interop
            })
            .await;

        Ok(controller)
    }
}

#[cfg(feature = "swift")]
pub mod swift_integration {
    use super::*;

    /// Setup auto-update for SwiftUI app
    pub async fn setup_swift_updater() -> anyhow::Result<UpdateController> {
        let controller = init_auto_update().await?;

        controller
            .set_ui_callback(move |event| {
                // Use Swift interop to send notification
                // Implementation depends on FFI bridge
            })
            .await;

        Ok(controller)
    }
}
