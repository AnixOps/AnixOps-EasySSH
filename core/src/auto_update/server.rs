//! Update server client for checking and downloading updates

use super::{UpdateChannel, UpdateInfo};
use reqwest::Client;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub struct UpdateServerClient {
    base_url: String,
    channel: UpdateChannel,
    client: Client,
}

impl UpdateServerClient {
    pub fn new(base_url: String, channel: UpdateChannel, timeout: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent(format!(
                "EasySSH/{} ({}; {})",
                super::CURRENT_VERSION,
                std::env::consts::OS,
                std::env::consts::ARCH,
            ))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url,
            channel,
            client,
        }
    }

    /// Check for available updates
    pub async fn check_update(
        &self,
        current_version: &str,
        channel_str: &str,
        ab_group: Option<&str>,
    ) -> anyhow::Result<Option<UpdateInfo>> {
        let mut params = HashMap::new();
        params.insert("version", current_version);
        params.insert("channel", channel_str);
        params.insert("os", std::env::consts::OS);
        params.insert("arch", std::env::consts::ARCH);

        if let Some(group) = ab_group {
            params.insert("ab_group", group);
        }

        // Get additional platform info
        #[cfg(target_os = "linux")]
        {
            let dist = super::platform::linux::get_distribution().await;
            let dist_ver = super::platform::linux::get_distribution_version().await;
            params.insert("distro", &dist);
            params.insert("distro_version", &dist_ver);
        }

        let url = format!("{}/api/v1/update/check", self.base_url);

        let response = self.client.get(&url).query(&params).send().await?;

        if response.status().is_success() {
            let update_info: Option<UpdateInfo> = response.json().await?;
            Ok(update_info)
        } else if response.status().as_u16() == 204 {
            // No update available
            Ok(None)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Server error {}: {}", status, body))
        }
    }

    /// Download update with progress callback
    pub async fn download_update<F>(
        &self,
        url: &str,
        destination: &Path,
        mut progress_callback: F,
    ) -> anyhow::Result<()>
    where
        F: FnMut(u64, u64),
    {
        let response = self.client.get(url).send().await?;

        let total_size = response.content_length().unwrap_or(0);

        let mut downloaded: u64 = 0;
        let mut file = tokio::fs::File::create(destination).await?;
        let mut stream = response.bytes_stream();

        use futures_util::TryStreamExt;

        while let Some(chunk) = stream.try_next().await? {
            let chunk_size = chunk.len() as u64;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk_size;
            progress_callback(downloaded, total_size);
        }

        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);

        Ok(())
    }

    /// Download signature file
    pub async fn download_signature(&self, url: &str, destination: &Path) -> anyhow::Result<()> {
        let response = self.client.get(url).send().await?;

        let bytes = response.bytes().await?;
        tokio::fs::write(destination, &bytes).await?;

        Ok(())
    }

    /// Get release notes
    pub async fn get_release_notes(&self, version: &str, locale: &str) -> anyhow::Result<String> {
        let url = format!("{}/api/v1/update/notes/{}", self.base_url, version);

        let response = self
            .client
            .get(&url)
            .query(&[("locale", locale)])
            .send()
            .await?;

        let notes = response.text().await?;
        Ok(notes)
    }

    /// Report update status (success/failure)
    pub async fn report_status(
        &self,
        install_id: &str,
        version: &str,
        success: bool,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/v1/update/report", self.base_url);

        let mut body = HashMap::new();
        body.insert("install_id", install_id);
        body.insert("version", version);
        body.insert("success", if success { "true" } else { "false" });

        if let Some(err) = error {
            body.insert("error", err);
        }

        self.client.post(&url).json(&body).send().await?;

        Ok(())
    }

    /// Get delta patch
    pub async fn get_delta_patch(
        &self,
        from_version: &str,
        to_version: &str,
    ) -> anyhow::Result<Option<String>> {
        let url = format!(
            "{}/api/v1/update/delta/{}/{}",
            self.base_url, from_version, to_version
        );

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["url"].as_str().map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Send heartbeat (for A/B testing and stats)
    pub async fn send_heartbeat(
        &self,
        install_id: &str,
        session_duration: u64,
        features_used: &[String],
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/v1/heartbeat", self.base_url);

        let body = serde_json::json!({
            "install_id": install_id,
            "version": super::CURRENT_VERSION,
            "channel": self.channel.to_string(),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "session_duration": session_duration,
            "features_used": features_used,
        });

        self.client.post(&url).json(&body).send().await?;

        Ok(())
    }
}

/// Update server response types
#[derive(Debug, serde::Deserialize)]
pub struct CheckUpdateResponse {
    pub update_available: bool,
    pub info: Option<UpdateInfo>,
    pub critical: bool,
    pub message: Option<String>,
}

/// Analytics data for update server
#[derive(Debug, serde::Serialize)]
pub struct UpdateAnalytics {
    pub install_id: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    pub update_method: String,
    pub success: bool,
    pub duration_seconds: u64,
    pub error_category: Option<String>,
}
