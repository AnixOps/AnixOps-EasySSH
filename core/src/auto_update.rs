//! Cross-platform auto-update system for EasySSH
//!
//! Features:
//! - Windows: WinSparkle-inspired implementation with background download
//! - macOS: Sparkle-inspired with notarization verification
//! - Linux: APT/RPM/DNF/AppImage support
//! - Update server with version checking and delta updates
//! - Signature verification (Ed25519)
//! - A/B testing support
//! - Rollback mechanism

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::interval;

pub mod platform;
pub mod server;
pub mod signature;
pub mod delta;
pub mod rollback;
pub mod ab_testing;
pub mod ui_integration;

pub use platform::PlatformUpdater;
pub use server::UpdateServerClient;
pub use signature::SignatureVerifier;
pub use delta::DeltaPatcher;
pub use rollback::RollbackManager;
pub use ab_testing::AbTestManager;
pub use ui_integration::{UpdateController, UpdateUiEvent, presets, init_auto_update};

/// Current application version
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Update channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
    Dev,
}

impl Default for UpdateChannel {
    fn default() -> Self {
        UpdateChannel::Stable
    }
}

impl std::fmt::Display for UpdateChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateChannel::Stable => write!(f, "stable"),
            UpdateChannel::Beta => write!(f, "beta"),
            UpdateChannel::Nightly => write!(f, "nightly"),
            UpdateChannel::Dev => write!(f, "dev"),
        }
    }
}

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Update server URL
    pub server_url: String,
    /// Update check interval (seconds)
    pub check_interval: u64,
    /// Update channel
    pub channel: UpdateChannel,
    /// Enable automatic download
    pub auto_download: bool,
    /// Enable automatic installation
    pub auto_install: bool,
    /// Show beta updates
    pub include_beta: bool,
    /// Silent mode (no UI notifications)
    pub silent_mode: bool,
    /// Signature public key (Ed25519 hex)
    pub signature_public_key: String,
    /// A/B test group (optional)
    pub ab_test_group: Option<String>,
    /// Installation directory
    pub install_dir: Option<PathBuf>,
    /// Temp directory for downloads
    pub temp_dir: Option<PathBuf>,
    /// Rollup backup count
    pub rollback_backup_count: u32,
    /// HTTP timeout (seconds)
    pub http_timeout: u64,
    /// Enable delta updates
    pub enable_delta: bool,
    /// Skip versions list
    pub skipped_versions: Vec<String>,
    /// Last check timestamp
    pub last_check: Option<u64>,
    /// Remind later timestamp
    pub remind_later_time: Option<u64>,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            server_url: "https://updates.easyssh.dev".to_string(),
            check_interval: 3600, // 1 hour
            channel: UpdateChannel::Stable,
            auto_download: true,
            auto_install: false,
            include_beta: false,
            silent_mode: false,
            signature_public_key: String::new(),
            ab_test_group: None,
            install_dir: None,
            temp_dir: None,
            rollback_backup_count: 3,
            http_timeout: 30,
            enable_delta: true,
            skipped_versions: Vec::new(),
            last_check: None,
            remind_later_time: None,
        }
    }
}

/// Update information from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub build_number: u32,
    pub release_notes: String,
    pub release_date: String,
    pub download_url: String,
    pub signature_url: String,
    pub size: u64,
    pub sha256: String,
    pub force_update: bool,
    pub min_version: Option<String>,
    pub delta_available: bool,
    pub delta_url: Option<String>,
    pub delta_size: Option<u64>,
    pub delta_from_version: Option<String>,
    pub platform: String,
    pub channel: String,
    pub ab_test_features: Option<Vec<String>>,
    pub rollout_percentage: Option<u8>,
}

/// Update progress
#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgress {
    pub stage: UpdateStage,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub speed_bps: f64,
    pub estimated_seconds: u64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UpdateStage {
    Idle,
    Checking,
    Downloading,
    Verifying,
    Patching,
    Installing,
    RollingBack,
    Complete,
    Error,
}

/// Update result
#[derive(Debug, Clone)]
pub enum UpdateResult {
    NoUpdateAvailable,
    UpdateAvailable(UpdateInfo),
    UpdateDownloaded { info: UpdateInfo, path: PathBuf },
    UpdateInstalled { version: String },
    UpdateFailed { error: String, can_rollback: bool },
    RolledBack { previous_version: String },
    Skipped { version: String },
    RemindLater { version: String },
}

/// User response to update prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateResponse {
    InstallNow,
    InstallLater,
    SkipVersion,
}

/// Auto updater state
#[derive(Debug)]
pub struct AutoUpdater {
    config: Arc<RwLock<UpdateConfig>>,
    state: Arc<RwLock<UpdaterState>>,
    server_client: UpdateServerClient,
    signature_verifier: Arc<RwLock<SignatureVerifier>>,
    rollback_manager: Arc<RollbackManager>,
    ab_manager: Arc<RwLock<AbTestManager>>,
    platform_updater: Arc<Box<dyn PlatformUpdater>>,
    progress_tx: mpsc::Sender<UpdateProgress>,
    progress_rx: Arc<Mutex<mpsc::Receiver<UpdateProgress>>>,
}

#[derive(Debug, Clone)]
struct UpdaterState {
    current_version: String,
    current_stage: UpdateStage,
    download_path: Option<PathBuf>,
    is_background_checking: bool,
    is_downloading: bool,
    is_installing: bool,
    pending_update: Option<UpdateInfo>,
}

impl AutoUpdater {
    /// Create new auto updater
    pub async fn new(config: UpdateConfig) -> anyhow::Result<Self> {
        let (progress_tx, progress_rx) = mpsc::channel(100);

        let signature_verifier = SignatureVerifier::new(&config.signature_public_key)?;
        let rollback_manager = RollbackManager::new(
            config.temp_dir.clone(),
            config.rollback_backup_count,
        ).await?;
        let ab_manager = AbTestManager::new(config.ab_test_group.clone()).await?;

        let platform_updater = platform::create_platform_updater().await?;

        let server_client = UpdateServerClient::new(
            config.server_url.clone(),
            config.channel,
            config.http_timeout,
        );

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(UpdaterState {
                current_version: CURRENT_VERSION.to_string(),
                current_stage: UpdateStage::Idle,
                download_path: None,
                is_background_checking: false,
                is_downloading: false,
                is_installing: false,
                pending_update: None,
            })),
            server_client,
            signature_verifier: Arc::new(RwLock::new(signature_verifier)),
            rollback_manager: Arc::new(rollback_manager),
            ab_manager: Arc::new(RwLock::new(ab_manager)),
            platform_updater: Arc::new(platform_updater),
            progress_tx,
            progress_rx: Arc::new(Mutex::new(progress_rx)),
        })
    }

    /// Start background update checker
    pub async fn start_background_checker(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(
            self.config.read().await.check_interval
        ));

        loop {
            interval.tick().await;

            let config = self.config.read().await;

            // Check if we should skip this check
            if let Some(remind_time) = config.remind_later_time {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if now < remind_time {
                    continue;
                }
            }

            drop(config);

            // Perform background check
            if let Err(e) = self.check_for_updates(true).await {
                log::warn!("Background update check failed: {}", e);
            }
        }
    }

    /// Check for updates
    pub async fn check_for_updates(&self, background: bool) -> anyhow::Result<UpdateResult> {
        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::Checking;
        state.is_background_checking = background;
        drop(state);

        self.send_progress(UpdateStage::Checking, 0, 0, 0.0, "Checking for updates...").await;

        let config = self.config.read().await;
        let current_version = &config.channel.to_string();
        let ab_group = config.ab_test_group.clone();
        drop(config);

        // Query update server
        let update_info = self.server_client.check_update(
            CURRENT_VERSION,
            current_version,
            ab_group.as_deref(),
        ).await?;

        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::Idle;
        state.is_background_checking = false;

        let result = if let Some(info) = update_info {
            // Check if version was skipped
            let config = self.config.read().await;
            if config.skipped_versions.contains(&info.version) {
                return Ok(UpdateResult::Skipped { version: info.version });
            }
            drop(config);

            // Check A/B rollout
            if let Some(percentage) = info.rollout_percentage {
                let ab_manager = self.ab_manager.read().await;
                if !ab_manager.is_in_rollout(&info.version, percentage) {
                    state.current_stage = UpdateStage::Idle;
                    return Ok(UpdateResult::NoUpdateAvailable);
                }
            }

            state.pending_update = Some(info.clone());
            UpdateResult::UpdateAvailable(info)
        } else {
            UpdateResult::NoUpdateAvailable
        };

        Ok(result)
    }

    /// Download update
    pub async fn download_update(&self, info: &UpdateInfo) -> anyhow::Result<PathBuf> {
        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::Downloading;
        state.is_downloading = true;
        drop(state);

        let temp_dir = self.get_temp_dir().await?;
        let download_path = temp_dir.join(format!(
            "easyssh-{}-{}.{}",
            info.version,
            info.build_number,
            self.platform_updater.get_package_extension()
        ));

        self.send_progress(UpdateStage::Downloading, 0, info.size, 0.0, "Downloading update...").await;

        // Download with progress
        let mut last_progress_time = std::time::Instant::now();
        let mut last_bytes = 0u64;

        self.server_client.download_update(
            &info.download_url,
            &download_path,
            |downloaded, total| {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_progress_time).as_secs_f64();

                if elapsed > 0.5 || downloaded == total {
                    let speed = (downloaded - last_bytes) as f64 / elapsed;
                    let percentage = if total > 0 {
                        (downloaded as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let estimated = if speed > 0.0 {
                        ((total - downloaded) as f64 / speed) as u64
                    } else {
                        0
                    };

                    let _ = self.progress_tx.try_send(UpdateProgress {
                        stage: UpdateStage::Downloading,
                        bytes_downloaded: downloaded,
                        total_bytes: total,
                        percentage: percentage as f32,
                        speed_bps: speed,
                        estimated_seconds: estimated,
                        message: format!("Downloading: {:.1}%", percentage),
                    });

                    last_progress_time = now;
                    last_bytes = downloaded;
                }
            },
        ).await?;

        // Verify SHA256
        self.send_progress(UpdateStage::Verifying, info.size, info.size, 100.0, "Verifying download...").await;

        let sha256 = self.calculate_sha256(&download_path).await?;
        if sha256 != info.sha256 {
            return Err(anyhow::anyhow!("SHA256 mismatch: expected {}, got {}", info.sha256, sha256));
        }

        // Download and verify signature
        let sig_path = temp_dir.join(format!("easyssh-{}-{}.sig", info.version, info.build_number));
        self.server_client.download_signature(&info.signature_url, &sig_path).await?;

        let sig_data = tokio::fs::read(&sig_path).await?;
        let package_data = tokio::fs::read(&download_path).await?;

        let verifier = self.signature_verifier.read().await;
        if !verifier.verify(&package_data, &sig_data)? {
            return Err(anyhow::anyhow!("Signature verification failed"));
        }
        drop(verifier);

        // Handle delta updates
        if info.delta_available && info.delta_url.is_some() {
            let delta_path = temp_dir.join(format!(
                "easyssh-{}-delta.patch",
                info.version
            ));

            self.send_progress(UpdateStage::Patching, 0, 100, 0.0, "Downloading delta patch...").await;

            self.server_client.download_update(
                info.delta_url.as_ref().unwrap(),
                &delta_path,
                |_, _| {}, // Silent download for delta
            ).await?;

            self.send_progress(UpdateStage::Patching, 50, 100, 50.0, "Applying delta patch...").await;

            // Apply delta patch to create full package
            let current_exe = std::env::current_exe()?;
            let delta_patcher = DeltaPatcher::new()?;
            delta_patcher.apply_patch(&current_exe, &delta_path, &download_path).await?;
        }

        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::Idle;
        state.is_downloading = false;
        state.download_path = Some(download_path.clone());

        Ok(download_path)
    }

    /// Install update
    pub async fn install_update(&self, info: &UpdateInfo, package_path: &Path) -> anyhow::Result<()> {
        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::Installing;
        state.is_installing = true;
        drop(state);

        self.send_progress(UpdateStage::Installing, 0, 100, 0.0, "Creating backup for rollback...").await;

        // Create rollback backup
        let current_exe = std::env::current_exe()?;
        self.rollback_manager.create_backup(&current_exe, &info.version).await?;

        self.send_progress(UpdateStage::Installing, 25, 100, 25.0, "Installing update...").await;

        // Platform-specific installation
        match self.platform_updater.install_update(package_path).await {
            Ok(_) => {
                let mut state = self.state.write().await;
                state.current_stage = UpdateStage::Complete;
                state.is_installing = false;
                Ok(())
            }
            Err(e) => {
                // Trigger rollback
                self.send_progress(UpdateStage::RollingBack, 0, 100, 0.0, "Installation failed, rolling back...").await;

                if let Err(rollback_err) = self.rollback_manager.rollback().await {
                    log::error!("Rollback failed: {}", rollback_err);
                }

                let mut state = self.state.write().await;
                state.current_stage = UpdateStage::Error;
                state.is_installing = false;

                Err(anyhow::anyhow!("Installation failed: {}", e))
            }
        }
    }

    /// Handle user response to update prompt
    pub async fn handle_user_response(&self, response: UpdateResponse, info: &UpdateInfo) -> anyhow::Result<UpdateResult> {
        match response {
            UpdateResponse::InstallNow => {
                let path = self.download_update(info).await?;
                self.install_update(info, &path).await?;
                Ok(UpdateResult::UpdateInstalled { version: info.version.clone() })
            }
            UpdateResponse::InstallLater => {
                let mut config = self.config.write().await;
                config.remind_later_time = Some(
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() + 86400 // 24 hours
                );
                drop(config);
                Ok(UpdateResult::RemindLater { version: info.version.clone() })
            }
            UpdateResponse::SkipVersion => {
                let mut config = self.config.write().await;
                if !config.skipped_versions.contains(&info.version) {
                    config.skipped_versions.push(info.version.clone());
                }
                drop(config);
                Ok(UpdateResult::Skipped { version: info.version.clone() })
            }
        }
    }

    /// Get update progress receiver
    pub async fn subscribe_progress(&self) -> mpsc::Receiver<UpdateProgress> {
        let (tx, rx) = mpsc::channel(100);

        // Take a clone of the Arc for the spawned task
        let progress_rx_arc = self.progress_rx.clone();

        // Spawn task to forward progress
        tokio::spawn(async move {
            // Lock inside the spawned task to avoid lifetime issues
            let mut internal_rx = progress_rx_arc.lock().await;
            while let Some(progress) = internal_rx.recv().await {
                let _ = tx.send(progress).await;
            }
        });

        rx
    }

    /// Get current state
    pub async fn get_state(&self) -> (UpdateStage, Option<UpdateInfo>) {
        let state = self.state.read().await;
        (state.current_stage.clone(), state.pending_update.clone())
    }

    /// Get current version
    pub fn get_current_version() -> &'static str {
        CURRENT_VERSION
    }

    /// Force rollback to previous version
    pub async fn rollback(&self) -> anyhow::Result<UpdateResult> {
        let mut state = self.state.write().await;
        state.current_stage = UpdateStage::RollingBack;
        drop(state);

        match self.rollback_manager.rollback().await {
            Ok(version) => {
                let mut state = self.state.write().await;
                state.current_stage = UpdateStage::Idle;
                Ok(UpdateResult::RolledBack { previous_version: version })
            }
            Err(e) => Err(e)
        }
    }

    /// Check if update is mandatory
    pub async fn is_mandatory_update(&self, info: &UpdateInfo) -> bool {
        if info.force_update {
            return true;
        }

        if let Some(min_version) = &info.min_version {
            return self.is_version_below_minimum(CURRENT_VERSION, min_version);
        }

        false
    }

    // Helper methods
    async fn get_temp_dir(&self) -> anyhow::Result<PathBuf> {
        let config = self.config.read().await;
        let temp_dir = config.temp_dir.clone()
            .unwrap_or_else(|| std::env::temp_dir().join("easyssh-updates"));
        drop(config);

        tokio::fs::create_dir_all(&temp_dir).await?;
        Ok(temp_dir)
    }

    async fn calculate_sha256(&self, path: &Path) -> anyhow::Result<String> {
        use sha2::{Sha256, Digest};
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    async fn send_progress(&self, stage: UpdateStage, bytes: u64, total: u64, percentage: f32, message: &str) {
        let _ = self.progress_tx.try_send(UpdateProgress {
            stage,
            bytes_downloaded: bytes,
            total_bytes: total,
            percentage,
            speed_bps: 0.0,
            estimated_seconds: 0,
            message: message.to_string(),
        });
    }

    fn is_version_below_minimum(&self, current: &str, minimum: &str) -> bool {
        match (semver::Version::parse(current), semver::Version::parse(minimum)) {
            (Ok(current), Ok(minimum)) => current < minimum,
            _ => false,
        }
    }
}

/// Check if auto update should be enabled
pub fn is_auto_update_enabled() -> bool {
    cfg!(feature = "auto-update")
}

/// Generate install UUID for tracking
pub fn generate_install_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallMetadata {
    pub install_id: String,
    pub first_install_version: String,
    pub first_install_date: String,
    pub platform: String,
    pub arch: String,
}

impl InstallMetadata {
    pub fn new() -> Self {
        Self {
            install_id: generate_install_id(),
            first_install_version: CURRENT_VERSION.to_string(),
            first_install_date: chrono::Local::now().to_rfc3339(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}
