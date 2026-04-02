//! Docker images - Image operations

use crate::error::LiteError;
use crate::ssh::SshSessionManager;
use tokio::sync::mpsc;

use super::client::DockerManager;
use super::types::ImageInfo;

impl DockerManager {
    /// List images
    pub async fn list_images(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        all: bool,
        dangling: bool,
    ) -> Result<Vec<ImageInfo>, LiteError> {
        let all_flag = if all { " -a" } else { "" };
        let filter_flag = if dangling { " --filter dangling=true" } else { "" };

        let cmd = format!(
            "docker images{} --format '{{{{json .}}}}' 2>/dev/null || docker images{}{} --format '{{{{.ID}}}}|{{{{.Repository}}}}|{{{{.Tag}}}}|{{{{.Size}}}}|{{{{.CreatedAt}}}}'",
            all_flag, all_flag, filter_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut images = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                images.push(self.parse_image_json(info)?);
            } else {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    images.push(ImageInfo {
                        id: parts[0].to_string(),
                        repo_tags: vec![format!("{}:{}", parts[1], parts[2])],
                        repo_digests: Vec::new(),
                        parent: String::new(),
                        comment: String::new(),
                        created: parts.get(4).unwrap_or(&"").to_string(),
                        container: String::new(),
                        size: self.parse_size(parts[3]),
                        virtual_size: 0,
                        shared_size: 0,
                        labels: std::collections::HashMap::new(),
                    });
                }
            }
        }

        Ok(images)
    }

    /// Pull image
    pub async fn pull_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image: &str,
        tag: Option<&str>,
        registry: Option<&str>,
    ) -> Result<(), LiteError> {
        let full_image = if let Some(reg) = registry {
            format!("{}/{}", reg, image)
        } else {
            image.to_string()
        };

        let full_image = if let Some(t) = tag {
            format!("{}:{}", full_image, t)
        } else {
            full_image
        };

        let cmd = format!("docker pull {}", full_image);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("Downloaded")
            || output.contains("up to date")
            || output.contains("Already exists")
        {
            Ok(())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to pull image: {}",
                output
            )))
        }
    }

    /// Remove image
    pub async fn remove_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image_id: &str,
        force: bool,
    ) -> Result<(), LiteError> {
        let force_flag = if force { " -f" } else { "" };
        let cmd = format!("docker rmi{} {}", force_flag, image_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("Deleted") || output.contains("Untagged") {
            Ok(())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to remove image: {}",
                output
            )))
        }
    }

    /// Tag image
    pub async fn tag_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        source: &str,
        target: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker tag {} {}", source, target);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to tag image: {}",
                output
            )))
        }
    }

    /// Push image
    pub async fn push_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker push {}", image);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("pushed") || output.contains("Layer already exists") {
            Ok(())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to push image: {}",
                output
            )))
        }
    }

    /// Inspect image
    pub async fn inspect_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image_id: &str,
    ) -> Result<ImageInfo, LiteError> {
        let cmd = format!("docker inspect {}", image_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if let Ok(info) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(arr) = info.as_array() {
                if let Some(first) = arr.first() {
                    return self.parse_image_inspect_json(first.clone());
                }
            }
        }

        Err(LiteError::Docker(format!(
            "Failed to inspect image: {}",
            output
        )))
    }

    /// Build image
    pub async fn build_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        context_path: &str,
        dockerfile_path: Option<&str>,
        tag: Option<&str>,
        build_args: &[(&str, &str)],
        no_cache: bool,
    ) -> Result<String, LiteError> {
        let dockerfile_flag = dockerfile_path
            .map(|d| format!(" -f {}", d))
            .unwrap_or_default();
        let tag_flag = tag.map(|t| format!(" -t {}", t)).unwrap_or_default();
        let no_cache_flag = if no_cache { " --no-cache" } else { "" };

        let mut build_args_flags = String::new();
        for (key, value) in build_args {
            build_args_flags.push_str(&format!(
                " --build-arg {}='{}'",
                key,
                value.replace("'", "'\\''")
            ));
        }

        let cmd = format!(
            "cd {} && docker build{} {} .{}{}",
            context_path, dockerfile_flag, tag_flag, no_cache_flag, build_args_flags
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        // Parse build output to find image ID
        let image_id = output
            .lines()
            .filter(|line| line.contains("Successfully built "))
            .last()
            .and_then(|line| line.split("Successfully built ").nth(1))
            .map(|s| s.trim().to_string());

        match image_id {
            Some(id) => Ok(id),
            None => {
                if output.contains("error") || output.contains("Error") {
                    Err(LiteError::Docker(format!("Build failed: {}", output)))
                } else {
                    // Try to find image ID from 'writing image' line
                    let img_id = output
                        .lines()
                        .filter(|line| line.contains("writing image "))
                        .last()
                        .and_then(|line| {
                            let start = line.find("sha256:")?;
                            let end = line[start..].find(' ').unwrap_or(line[start..].len());
                            Some(line[start..start + end].to_string())
                        });

                    match img_id {
                        Some(id) => Ok(id),
                        None => Err(LiteError::Docker(format!(
                            "Build output unclear: {}",
                            output
                        ))),
                    }
                }
            }
        }
    }

    /// Parse image inspect JSON
    pub fn parse_image_inspect_json(&self, value: serde_json::Value) -> Result<ImageInfo, LiteError> {
        let config = value.get("Config").and_then(|v| v.as_object());

        Ok(ImageInfo {
            id: value.get("Id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            repo_tags: value
                .get("RepoTags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            repo_digests: value
                .get("RepoDigests")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            parent: value
                .get("Parent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            comment: value
                .get("Comment")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created: value
                .get("Created")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            container: value
                .get("Container")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            size: value.get("Size").and_then(|v| v.as_i64()).unwrap_or(0),
            virtual_size: value.get("VirtualSize").and_then(|v| v.as_i64()).unwrap_or(0),
            shared_size: 0,
            labels: config
                .and_then(|c| c.get("Labels"))
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}
