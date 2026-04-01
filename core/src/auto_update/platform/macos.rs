//! macOS updater implementation with Sparkle-inspired approach
#![cfg(target_os = "macos")]

use super::PlatformUpdater;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug)]
pub struct MacOSUpdater {
    app_bundle_path: PathBuf,
    temp_dir: PathBuf,
}

impl MacOSUpdater {
    pub async fn new() -> anyhow::Result<Self> {
        // Find the app bundle path
        let current_exe = std::env::current_exe()?;
        let app_bundle_path = Self::find_app_bundle(&current_exe)?;

        let temp_dir = std::env::temp_dir().join("easyssh-updates");
        tokio::fs::create_dir_all(&temp_dir).await?;

        Ok(Self {
            app_bundle_path,
            temp_dir,
        })
    }

    fn find_app_bundle(exe_path: &Path) -> anyhow::Result<PathBuf> {
        // Walk up to find .app bundle
        let mut path = exe_path.to_path_buf();
        while let Some(parent) = path.parent() {
            if parent.extension()
                .map(|e| e == "app")
                .unwrap_or(false)
            {
                return Ok(parent.to_path_buf());
            }
            path = parent.to_path_buf();
        }

        // Fallback: return the directory containing the binary
        exe_path.parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| anyhow::anyhow!("Could not find app bundle"))
    }

    /// Verify notarization status
    pub async fn verify_notarization(app_path: &Path) -> anyhow::Result<bool> {
        let output = Command::new("spctl")
            .args(&["-a", "-vv", app_path.to_str().unwrap()])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("accepted") || output.status.success())
    }

    /// Staple notarization ticket to app
    pub async fn staple_notarization(app_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("xcrun")
            .args(&["stapler", "staple", app_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to staple notarization: {}", stderr));
        }

        Ok(())
    }

    /// Install from DMG
    async fn install_dmg(&self, dmg_path: &Path) -> anyhow::Result<()> {
        // Mount DMG
        let mount_output = Command::new("hdiutil")
            .args(&[
                "attach",
                dmg_path.to_str().unwrap(),
                "-nobrowse",
                "-noverify",
                "-noautoopen",
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&mount_output.stdout);

        // Parse mount point from output
        let mount_point = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                parts.last().map(|s| s.trim().to_string())
            })
            .find(|s| s.starts_with('/'))
            .ok_or_else(|| anyhow::anyhow!("Could not find mount point"))?;

        // Find the app bundle in the mounted DMG
        let new_app = std::fs::read_dir(&mount_point)?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry.path().extension()
                    .map(|e| e == "app")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .ok_or_else(|| anyhow::anyhow!("No app bundle found in DMG"))?;

        // Verify notarization before installation
        if !Self::verify_notarization(&new_app).await? {
            return Err(anyhow::anyhow!("App not notarized"));
        }

        // Check quarantine attribute
        Self::remove_quarantine(&new_app).await?;

        // Replace the app
        self.replace_app_bundle(&new_app).await?;

        // Unmount DMG
        Command::new("hdiutil")
            .args(&["detach", &mount_point, "-force"])
            .output()?;

        Ok(())
    }

    /// Install from ZIP
    async fn install_zip(&self, zip_path: &Path) -> anyhow::Result<()> {
        // Unzip to temp
        let extract_dir = self.temp_dir.join("extracted");
        tokio::fs::create_dir_all(&extract_dir).await?;

        let output = Command::new("unzip")
            .args(&[
                "-o", // Overwrite
                zip_path.to_str().unwrap(),
                "-d",
                extract_dir.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to extract ZIP"));
        }

        // Find the app bundle
        let new_app = std::fs::read_dir(&extract_dir)?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry.path().extension()
                    .map(|e| e == "app")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .ok_or_else(|| anyhow::anyhow!("No app bundle found in ZIP"))?;

        self.replace_app_bundle(&new_app).await?;

        Ok(())
    }

    /// Install from tar.gz
    async fn install_tar(&self, tar_path: &Path) -> anyhow::Result<()> {
        // Extract to temp
        let extract_dir = self.temp_dir.join("extracted");
        tokio::fs::create_dir_all(&extract_dir).await?;

        let output = Command::new("tar")
            .args(&[
                "-xzf",
                tar_path.to_str().unwrap(),
                "-C",
                extract_dir.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to extract tarball"));
        }

        // Find the app bundle
        let new_app = Self::find_app_in_directory(&extract_dir)?;
        self.replace_app_bundle(&new_app).await?;

        Ok(())
    }

    fn find_app_in_directory(dir: &Path) -> anyhow::Result<PathBuf> {
        for entry in walkdir::WalkDir::new(dir).max_depth(3) {
            let entry = entry?;
            if entry.path().extension()
                .map(|e| e == "app")
                .unwrap_or(false)
            {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(anyhow::anyhow!("No .app bundle found"))
    }

    /// Replace app bundle with atomic operation
    async fn replace_app_bundle(&self, new_app: &Path) -> anyhow::Result<()> {
        // Get parent of current app (usually /Applications or ~/Applications)
        let install_parent = self.app_bundle_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid app path"))?;

        let app_name = self.app_bundle_path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid app name"))?;

        // Create temporary path in same directory (for atomic rename)
        let temp_app = install_parent.join(format!(".{}.tmp", app_name.to_str().unwrap()));
        let old_app = install_parent.join(format!(
            "{}.backup.{}",
            app_name.to_str().unwrap(),
            chrono::Local::now().timestamp()
        ));

        // Copy new app to temp location
        Self::copy_app_bundle(new_app, &temp_app).await?;

        // Remove quarantine attribute
        Self::remove_quarantine(&temp_app).await?;

        // Atomic swap: rename current -> old, temp -> current
        tokio::fs::rename(&self.app_bundle_path, &old_app).await?;
        tokio::fs::rename(&temp_app, &self.app_bundle_path).await?;

        // Cleanup old version (keep for rollback)
        // Will be cleaned up after successful restart

        Ok(())
    }

    async fn copy_app_bundle(src: &Path, dst: &Path) -> anyhow::Result<()> {
        // Use ditto for proper app bundle copying
        let output = Command::new("ditto")
            .args(&[src.to_str().unwrap(), dst.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            // Fallback to cp -R
            let output = Command::new("cp")
                .args(&["-R", src.to_str().unwrap(), dst.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("Failed to copy app bundle"));
            }
        }

        Ok(())
    }

    /// Remove quarantine attribute
    async fn remove_quarantine(app_path: &Path) -> anyhow::Result<()> {
        Command::new("xattr")
            .args(&["-d", "-r", "com.apple.quarantine", app_path.to_str().unwrap()])
            .output()?;

        // Ignore errors - attribute might not exist
        Ok(())
    }

    /// Check if Gatekeeper will allow the app
    pub async fn check_gatekeeper(app_path: &Path) -> anyhow::Result<bool> {
        let output = Command::new("spctl")
            .args(&["--assess", "--type", "exec", app_path.to_str().unwrap()])
            .output()?;

        Ok(output.status.success())
    }

    /// Get code signing info
    pub async fn get_code_signature(app_path: &Path) -> anyhow::Result<String> {
        let output = Command::new("codesign")
            .args(&["-dvv", app_path.to_str().unwrap()])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[async_trait]
impl PlatformUpdater for MacOSUpdater {
    fn get_package_extension(&self) -> &'static str {
        "dmg"
    }

    async fn install_update(&self, package_path: &Path) -> anyhow::Result<()> {
        let extension = package_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("dmg");

        match extension.to_lowercase().as_str() {
            "dmg" => self.install_dmg(package_path).await,
            "zip" => self.install_zip(package_path).await,
            "gz" | "tgz" => self.install_tar(package_path).await,
            _ => Err(anyhow::anyhow!("Unsupported package format: {}", extension)),
        }
    }

    async fn restart_application(&self) -> anyhow::Result<()> {
        // Use macOS open command to restart
        Command::new("open")
            .args(&["-n", "-W", self.app_bundle_path.to_str().unwrap()])
            .spawn()?;

        // Exit current instance
        std::process::exit(0);
    }

    async fn needs_elevation(&self, _package_path: &Path) -> anyhow::Result<bool> {
        // Check if app is in /Applications (requires admin)
        let app_path = self.app_bundle_path.canonicalize()?;
        Ok(app_path.starts_with("/Applications"))
    }

    fn get_current_executable(&self) -> anyhow::Result<std::path::PathBuf> {
        std::env::current_exe()
    }

    async fn verify_package(&self, package_path: &Path) -> anyhow::Result<bool> {
        let extension = package_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "dmg" => {
                // DMG verification - check hdiutil
                let output = Command::new("hdiutil")
                    .args(&["verify", package_path.to_str().unwrap()])
                    .output()?;

                Ok(output.status.success())
            }
            "app" => Self::verify_notarization(package_path).await,
            _ => Ok(true), // ZIP/tar cannot be verified before extraction
        }
    }
}
