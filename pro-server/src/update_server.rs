//! Update Server Reference Implementation
//! This is a reference implementation for the EasySSH update server

use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Update server state
pub struct UpdateServer {
    /// Available releases by version
    releases: Arc<RwLock<HashMap<String, Release>>>,
    /// Delta patches
    deltas: Arc<RwLock<HashMap<(String, String), DeltaInfo>>>,
    /// Feature flags
    feature_flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
    /// A/B test groups
    ab_groups: Arc<RwLock<HashMap<String, AbGroup>>>,
    /// Rollout percentages
    rollouts: Arc<RwLock<HashMap<String, u8>>>,
    /// CDN base URL
    cdn_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub build_number: u32,
    pub release_notes: String,
    pub release_date: String,
    pub force_update: bool,
    pub min_version: Option<String>,
    pub platforms: HashMap<String, PlatformPackage>,
    pub channels: Vec<String>,
    pub rollout_percentage: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPackage {
    pub url: String,
    pub signature_url: String,
    pub size: u64,
    pub sha256: String,
    pub delta_available: bool,
    pub delta_url: Option<String>,
    pub delta_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaInfo {
    pub from_version: String,
    pub to_version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub rollout_percentage: u8,
    pub target_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbGroup {
    pub name: String,
    pub features: Vec<String>,
    pub rollout_modifier: f32, // Multiplier for rollout percentage
}

/// Query parameters for update check
#[derive(Debug, Deserialize)]
pub struct CheckUpdateQuery {
    pub version: String,
    pub channel: String,
    pub os: String,
    pub arch: String,
    #[serde(default)]
    pub distro: Option<String>,
    #[serde(default)]
    pub distro_version: Option<String>,
    #[serde(default)]
    pub ab_group: Option<String>,
    #[serde(default)]
    pub install_id: Option<String>,
}

/// Update check response
#[derive(Debug, Serialize)]
pub struct CheckUpdateResponse {
    pub update_available: bool,
    pub info: Option<UpdateInfo>,
    pub critical: bool,
    pub message: Option<String>,
}

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

/// Report update status request
#[derive(Debug, Deserialize)]
pub struct ReportUpdateRequest {
    pub install_id: String,
    pub version: String,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_seconds: u64,
    pub platform: String,
    pub update_method: String,
}

/// Heartbeat request
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub install_id: String,
    pub version: String,
    pub channel: String,
    pub os: String,
    pub arch: String,
    pub session_duration: u64,
    pub features_used: Vec<String>,
}

/// Heartbeat response
#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub feature_flags: HashMap<String, bool>,
    pub rollouts: HashMap<String, u8>,
}

impl UpdateServer {
    pub fn new(cdn_url: String) -> Self {
        Self {
            releases: Arc::new(RwLock::new(HashMap::new())),
            deltas: Arc::new(RwLock::new(HashMap::new())),
            feature_flags: Arc::new(RwLock::new(HashMap::new())),
            ab_groups: Arc::new(RwLock::new(HashMap::new())),
            rollouts: Arc::new(RwLock::new(HashMap::new())),
            cdn_url,
        }
    }

    /// Check for available update
    pub async fn check_update(
        &self,
        query: &CheckUpdateQuery,
    ) -> anyhow::Result<Option<UpdateInfo>> {
        let releases = self.releases.read().await;

        // Find latest release for channel
        let latest = releases
            .values()
            .filter(|r| r.channels.contains(&query.channel))
            .filter(|r| r.platforms.contains_key(&query.os))
            .max_by(|a, b| {
                semver::Version::parse(&a.version)
                    .unwrap()
                    .cmp(&semver::Version::parse(&b.version).unwrap())
            });

        let latest = match latest {
            Some(l) => l,
            None => return Ok(None),
        };

        // Check if update is needed
        let current_version = semver::Version::parse(&query.version)?;
        let latest_version = semver::Version::parse(&latest.version)?;

        if latest_version <= current_version {
            return Ok(None);
        }

        // Check rollout percentage
        let rollout_pct = self.get_effective_rollout(&latest.version, &query.install_id).await;
        if rollout_pct < 100 {
            // User not in rollout
            if let Some(ref install_id) = query.install_id {
                if !self.is_in_rollout(install_id, &latest.version, rollout_pct) {
                    return Ok(None);
                }
            }
        }

        // Get platform package
        let platform_pkg = latest.platforms.get(&query.os)
            .ok_or_else(|| anyhow::anyhow!("Platform not found"))?;

        // Determine delta availability
        let (delta_available, delta_url, delta_size) =
            if let Some(ref install_id) = query.install_id {
                self.check_delta(&query.version, &latest.version, &query.os, install_id).await
            } else {
                (platform_pkg.delta_available, platform_pkg.delta_url.clone(), platform_pkg.delta_size)
            };

        // Get A/B test features
        let ab_features = if let Some(ref group) = query.ab_group {
            self.get_ab_features(group).await
        } else {
            None
        };

        Ok(Some(UpdateInfo {
            version: latest.version.clone(),
            build_number: latest.build_number,
            release_notes: latest.release_notes.clone(),
            release_date: latest.release_date.clone(),
            download_url: platform_pkg.url.clone(),
            signature_url: platform_pkg.signature_url.clone(),
            size: platform_pkg.size,
            sha256: platform_pkg.sha256.clone(),
            force_update: latest.force_update,
            min_version: latest.min_version.clone(),
            delta_available,
            delta_url,
            delta_size,
            delta_from_version: Some(query.version.clone()),
            platform: query.os.clone(),
            channel: query.channel.clone(),
            ab_test_features: ab_features,
            rollout_percentage: Some(rollout_pct),
        }))
    }

    /// Calculate effective rollout percentage
    async fn get_effective_rollout(&self, version: &str, _install_id: &Option<String>) -> u8 {
        let rollouts = self.rollouts.read().await;
        rollouts.get(version).copied().unwrap_or(100)
    }

    /// Check if install is in rollout bucket
    fn is_in_rollout(&self, install_id: &str, version: &str, percentage: u8) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{}:{}", install_id, version).hash(&mut hasher);
        let hash = hasher.finish();

        let bucket = (hash % 100) as u8;
        bucket < percentage
    }

    /// Check for available delta
    async fn check_delta(
        &self,
        from_version: &str,
        to_version: &str,
        platform: &str,
        _install_id: &str,
    ) -> (bool, Option<String>, Option<u64>) {
        let deltas = self.deltas.read().await;

        if let Some(delta) = deltas.get(&(from_version.to_string(), to_version.to_string())) {
            (
                true,
                Some(delta.url.clone()),
                Some(delta.size),
            )
        } else {
            // Check platform delta
            let releases = self.releases.read().await;
            if let Some(release) = releases.get(to_version) {
                if let Some(pkg) = release.platforms.get(platform) {
                    return (
                        pkg.delta_available,
                        pkg.delta_url.clone(),
                        pkg.delta_size,
                    );
                }
            }

            (false, None, None)
        }
    }

    /// Get A/B test features for group
    async fn get_ab_features(&self, group: &str) -> Option<Vec<String>> {
        let groups = self.ab_groups.read().await;
        groups.get(group).map(|g| g.features.clone())
    }

    /// Handle update report
    pub async fn report_update(&self, report: &ReportUpdateRequest) -> anyhow::Result<()> {
        // Log update result
        if report.success {
            log::info!(
                "Update successful: {} -> {} ({} seconds)",
                report.install_id,
                report.version,
                report.duration_seconds
            );
        } else {
            log::warn!(
                "Update failed: {} -> {} - {:?}",
                report.install_id,
                report.version,
                report.error
            );
        }

        // Update metrics
        // In production, send to analytics/metrics system

        Ok(())
    }

    /// Handle heartbeat
    pub async fn heartbeat(&self, req: &HeartbeatRequest) -> HeartbeatResponse {
        let flags = self.feature_flags.read().await;
        let rollouts = self.rollouts.read().await;

        // Determine which feature flags are enabled for this user
        let mut enabled_flags = HashMap::new();
        for (name, flag) in flags.iter() {
            let enabled = if flag.enabled {
                if flag.rollout_percentage < 100 {
                    self.is_in_rollout(&req.install_id, name, flag.rollout_percentage)
                } else {
                    true
                }
            } else {
                false
            };

            if enabled || flag.target_groups.contains(&req.channel) {
                enabled_flags.insert(name.clone(), true);
            }
        }

        HeartbeatResponse {
            feature_flags: enabled_flags,
            rollouts: rollouts.clone(),
        }
    }
}

/// HTTP handlers
mod handlers {
    use super::*;
    use actix_web::{web, HttpResponse, Result};

    pub async fn check_update(
        server: web::Data<UpdateServer>,
        query: web::Query<CheckUpdateQuery>,
    ) -> Result<HttpResponse> {
        match server.check_update(&query).await {
            Ok(Some(info)) => {
                let response = CheckUpdateResponse {
                    update_available: true,
                    info: Some(info),
                    critical: false,
                    message: None,
                };
                Ok(HttpResponse::Ok().json(response))
            }
            Ok(None) => Ok(HttpResponse::NoContent().finish()),
            Err(e) => {
                log::error!("Check update error: {}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        }
    }

    pub async fn report_update(
        server: web::Data<UpdateServer>,
        report: web::Json<ReportUpdateRequest>,
    ) -> Result<HttpResponse> {
        match server.report_update(&report).await {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(e) => {
                log::error!("Report update error: {}", e);
                Ok(HttpResponse::InternalServerError().finish())
            }
        }
    }

    pub async fn heartbeat(
        server: web::Data<UpdateServer>,
        req: web::Json<HeartbeatRequest>,
    ) -> Result<HttpResponse> {
        let response = server.heartbeat(&req).await;
        Ok(HttpResponse::Ok().json(response))
    }
}

/// Server configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub cdn_url: String,
    pub database_url: String,
}

/// Start the update server
pub async fn start_server(config: ServerConfig) -> anyhow::Result<()> {
    let server = Arc::new(UpdateServer::new(config.cdn_url));

    log::info!("Starting update server on {}:{}", config.bind_address, config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(server.clone()))
            .route("/api/v1/update/check", web::get().to(handlers::check_update))
            .route("/api/v1/update/report", web::post().to(handlers::report_update))
            .route("/api/v1/heartbeat", web::post().to(handlers::heartbeat))
    })
    .bind((config.bind_address.as_str(), config.port))?
    .run()
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rollout_calculation() {
        let server = UpdateServer::new("https://cdn.example.com".to_string());

        // Same install_id + version should always give same result
        let result1 = server.is_in_rollout("test-id", "1.0.0", 50);
        let result2 = server.is_in_rollout("test-id", "1.0.0", 50);
        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_check_update() {
        let server = UpdateServer::new("https://cdn.example.com".to_string());

        // Add a test release
        {
            let mut releases = server.releases.write().await;
            releases.insert("1.0.0".to_string(), Release {
                version: "1.0.0".to_string(),
                build_number: 100,
                release_notes: "Test release".to_string(),
                release_date: "2026-03-31".to_string(),
                force_update: false,
                min_version: None,
                platforms: {
                    let mut map = HashMap::new();
                    map.insert("windows".to_string(), PlatformPackage {
                        url: "https://cdn.example.com/1.0.0/easyssh.msi".to_string(),
                        signature_url: "https://cdn.example.com/1.0.0/easyssh.msi.sig".to_string(),
                        size: 50000000,
                        sha256: "abc123".to_string(),
                        delta_available: false,
                        delta_url: None,
                        delta_size: None,
                    });
                    map
                },
                channels: vec!["stable".to_string()],
                rollout_percentage: 100,
            });
        }

        let query = CheckUpdateQuery {
            version: "0.9.0".to_string(),
            channel: "stable".to_string(),
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            distro: None,
            distro_version: None,
            ab_group: None,
            install_id: Some("test-id".to_string()),
        };

        let result = server.check_update(&query).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}
