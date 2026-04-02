//! Sync providers for different backends

use crate::error::LiteError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

use super::types::{DeviceInfo, SyncBundle, SyncConfig, SyncMetadata, SyncVersion};

/// Sync provider trait
#[async_trait]
pub trait SyncProviderImpl: Send + Sync {
    async fn initialize(&mut self, config: &SyncConfig) -> Result<(), LiteError>;
    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError>;
    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError>;
    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError>;
    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError>;
    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError>;
    async fn delete_device(&self, device_id: &str) -> Result<(), LiteError>;
    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError>;
    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError>;
    async fn check_connectivity(&self) -> Result<bool, LiteError>;
}

/// Local sync handler
pub struct LocalSyncHandler {
    enabled: bool,
    port: u16,
    discovered_devices: HashMap<String, DeviceInfo>,
    last_beacon_sent: i64,
}

impl LocalSyncHandler {
    pub fn new() -> Self {
        Self {
            enabled: false,
            port: 0,
            discovered_devices: HashMap::new(),
            last_beacon_sent: 0,
        }
    }

    pub fn enable(&mut self, port: u16) {
        self.enabled = true;
        self.port = port;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn discover_devices(&self) -> Vec<DeviceInfo> {
        self.discovered_devices.values().cloned().collect()
    }

    pub fn add_discovered_device(&mut self, device: DeviceInfo) {
        self.discovered_devices
            .insert(device.device_id.clone(), device);
    }

    pub fn remove_device(&mut self, device_id: &str) {
        self.discovered_devices.remove(device_id);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Disabled provider
pub struct DisabledProvider;

impl DisabledProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SyncProviderImpl for DisabledProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Sync is disabled".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(false)
    }
}

/// iCloud provider
pub struct ICloudProvider {
    container_url: Option<PathBuf>,
}

impl ICloudProvider {
    pub fn new() -> Self {
        Self {
            container_url: None,
        }
    }
}

#[async_trait]
impl SyncProviderImpl for ICloudProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        tracing::debug!("Uploading bundle to iCloud: {}", bundle.bundle_id);
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        tracing::debug!("Downloading bundles from iCloud since {}", since);
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(self.container_url.is_some())
    }
}

/// Google Drive provider
pub struct GoogleDriveProvider {
    access_token: Option<String>,
    folder_id: Option<String>,
}

impl GoogleDriveProvider {
    pub fn new() -> Self {
        Self {
            access_token: None,
            folder_id: None,
        }
    }
}

#[async_trait]
impl SyncProviderImpl for GoogleDriveProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        tracing::debug!("Uploading bundle to Google Drive: {}", bundle.bundle_id);
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(self.access_token.is_some())
    }
}

/// OneDrive provider
pub struct OneDriveProvider {
    access_token: Option<String>,
}

impl OneDriveProvider {
    pub fn new() -> Self {
        Self { access_token: None }
    }
}

#[async_trait]
impl SyncProviderImpl for OneDriveProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(true)
    }
}

/// Dropbox provider
pub struct DropBoxProvider;

impl DropBoxProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SyncProviderImpl for DropBoxProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config("Not implemented".to_string()))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(true)
    }
}

/// Self-hosted provider
pub struct SelfHostedProvider {
    url: String,
    token: String,
    client: reqwest::Client,
}

impl SelfHostedProvider {
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SyncProviderImpl for SelfHostedProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        tracing::debug!("Initializing self-hosted provider: {}", self.url);
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/bundle", self.url);
        let _response = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(bundle)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        let url = format!("{}/api/v1/sync/bundles?since={}", self.url, since);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let bundles: Vec<SyncBundle> = response
            .json()
            .await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundles)
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        let url = format!("{}/api/v1/sync/metadata", self.url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let metadata: SyncMetadata = response
            .json()
            .await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(metadata)
    }

    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/metadata", self.url);
        let _response = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(metadata)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        let url = format!("{}/api/v1/sync/devices", self.url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let devices: Vec<DeviceInfo> = response
            .json()
            .await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(devices)
    }

    async fn delete_device(&self, device_id: &str) -> Result<(), LiteError> {
        let url = format!("{}/api/v1/sync/devices/{}", self.url, device_id);
        let _response = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        let url = format!("{}/api/v1/sync/versions", self.url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let versions: Vec<SyncVersion> = response
            .json()
            .await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(versions)
    }

    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError> {
        let url = format!("{}/api/v1/sync/versions/{}/restore", self.url, version_id);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LiteError::Config(e.to_string()))?;

        let bundle: SyncBundle = response
            .json()
            .await
            .map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundle)
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        let url = format!("{}/api/v1/health", self.url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Local network provider
pub struct LocalNetworkProvider;

impl LocalNetworkProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SyncProviderImpl for LocalNetworkProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        Ok(())
    }

    async fn upload_bundle(&self, _bundle: &SyncBundle) -> Result<(), LiteError> {
        Ok(())
    }

    async fn download_bundles(&self, _since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        Ok(Vec::new())
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, _metadata: &SyncMetadata) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, _version_id: &str) -> Result<SyncBundle, LiteError> {
        Err(LiteError::Config(
            "Local sync doesn't support versions".to_string(),
        ))
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(true)
    }
}

/// Local file provider (for testing or NAS)
pub struct LocalFileProvider {
    base_path: PathBuf,
}

impl LocalFileProvider {
    pub fn new(path: PathBuf) -> Self {
        Self { base_path: path }
    }

    fn get_bundle_path(&self, bundle_id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", bundle_id))
    }

    fn get_metadata_path(&self) -> PathBuf {
        self.base_path.join("metadata.json")
    }
}

#[async_trait]
impl SyncProviderImpl for LocalFileProvider {
    async fn initialize(&mut self, _config: &SyncConfig) -> Result<(), LiteError> {
        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn upload_bundle(&self, bundle: &SyncBundle) -> Result<(), LiteError> {
        let path = self.get_bundle_path(&bundle.bundle_id);
        let data = serde_json::to_vec_pretty(bundle)?;
        tokio::fs::write(path, data)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn download_bundles(&self, since: i64) -> Result<Vec<SyncBundle>, LiteError> {
        let mut bundles = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(data) = tokio::fs::read(&path).await {
                    if let Ok(bundle) = serde_json::from_slice::<SyncBundle>(&data) {
                        if bundle.timestamp > since {
                            bundles.push(bundle);
                        }
                    }
                }
            }
        }

        bundles.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(bundles)
    }

    async fn get_metadata(&self) -> Result<SyncMetadata, LiteError> {
        let path = self.get_metadata_path();
        if let Ok(data) = tokio::fs::read(&path).await {
            if let Ok(metadata) = serde_json::from_slice::<SyncMetadata>(&data) {
                return Ok(metadata);
            }
        }
        Ok(SyncMetadata::default())
    }

    async fn update_metadata(&self, metadata: &SyncMetadata) -> Result<(), LiteError> {
        let path = self.get_metadata_path();
        let data = serde_json::to_vec_pretty(metadata)?;
        tokio::fs::write(path, data)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        Ok(())
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, LiteError> {
        Ok(Vec::new())
    }

    async fn delete_device(&self, _device_id: &str) -> Result<(), LiteError> {
        Ok(())
    }

    async fn list_versions(&self) -> Result<Vec<SyncVersion>, LiteError> {
        Ok(Vec::new())
    }

    async fn restore_version(&self, version_id: &str) -> Result<SyncBundle, LiteError> {
        let path = self.get_bundle_path(version_id);
        let data = tokio::fs::read(&path)
            .await
            .map_err(|e| LiteError::Io(e.to_string()))?;
        let bundle = serde_json::from_slice(&data).map_err(|e| LiteError::Json(e.to_string()))?;
        Ok(bundle)
    }

    async fn check_connectivity(&self) -> Result<bool, LiteError> {
        Ok(self.base_path.exists())
    }
}
