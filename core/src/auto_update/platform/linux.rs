//! Linux updater implementation supporting multiple package formats
#![cfg(target_os = "linux")]

use super::PlatformUpdater;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    AppImage,
    Flatpak,
    Snap,
    Portable,
}

#[derive(Debug)]
pub struct LinuxUpdater {
    package_manager: PackageManager,
    temp_dir: PathBuf,
    install_dir: PathBuf,
}

impl LinuxUpdater {
    pub async fn new() -> anyhow::Result<Self> {
        let temp_dir = std::env::temp_dir().join("easyssh-updates");
        tokio::fs::create_dir_all(&temp_dir).await?;

        let package_manager = Self::detect_package_manager().await;

        let install_dir = match package_manager {
            PackageManager::AppImage => dirs::executable_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local/bin")),
            PackageManager::Flatpak => PathBuf::from("/var/lib/flatpak"),
            PackageManager::Snap => PathBuf::from("/snap"),
            _ => PathBuf::from("/usr/bin"),
        };

        Ok(Self {
            package_manager,
            temp_dir,
            install_dir,
        })
    }

    async fn detect_package_manager() -> PackageManager {
        // Check if we're an AppImage
        if std::env::var("APPIMAGE").is_ok() {
            return PackageManager::AppImage;
        }

        // Check for Flatpak
        if Path::new("/.flatpak-info").exists() {
            return PackageManager::Flatpak;
        }

        // Check for Snap
        if std::env::var("SNAP").is_ok() {
            return PackageManager::Snap;
        }

        // Check system package managers
        let managers = [
            (PackageManager::Apt, "/usr/bin/apt"),
            (PackageManager::Dnf, "/usr/bin/dnf"),
            (PackageManager::Yum, "/usr/bin/yum"),
            (PackageManager::Pacman, "/usr/bin/pacman"),
            (PackageManager::Zypper, "/usr/bin/zypper"),
        ];

        for (pm, path) in &managers {
            if Path::new(path).exists() {
                return *pm;
            }
        }

        // Default to portable
        PackageManager::Portable
    }

    /// Install using APT
    async fn install_apt(&self, deb_path: &Path) -> anyhow::Result<()> {
        // Install .deb package
        let output = Command::new("sudo")
            .args(&["dpkg", "-i", deb_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            // Try to fix broken dependencies
            Command::new("sudo")
                .args(&["apt-get", "install", "-f", "-y"])
                .output()?;

            // Retry
            let output = Command::new("sudo")
                .args(&["dpkg", "-i", deb_path.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("APT installation failed"));
            }
        }

        Ok(())
    }

    /// Install using DNF/YUM
    async fn install_dnf(&self, rpm_path: &Path) -> anyhow::Result<()> {
        let pm = if self.package_manager == PackageManager::Dnf {
            "dnf"
        } else {
            "yum"
        };

        let output = Command::new("sudo")
            .args(&[pm, "install", "-y", rpm_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("{} installation failed: {}", pm, stderr));
        }

        Ok(())
    }

    /// Install using Pacman
    async fn install_pacman(&self, pkg_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("sudo")
            .args(&["pacman", "-U", "--noconfirm", pkg_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Pacman installation failed"));
        }

        Ok(())
    }

    /// Install using Zypper
    async fn install_zypper(&self, rpm_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("sudo")
            .args(&["zypper", "install", "-y", rpm_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Zypper installation failed"));
        }

        Ok(())
    }

    /// Install AppImage
    async fn install_appimage(&self, appimage_path: &Path) -> anyhow::Result<()> {
        let target_path = self.install_dir.join("EasySSH.AppImage");

        // Copy AppImage to install location
        tokio::fs::copy(appimage_path, &target_path).await?;

        // Make executable
        tokio::fs::set_permissions(
            &target_path,
            std::fs::Permissions::from_mode(0o755),
        ).await?;

        // Update desktop integration
        self.update_desktop_integration(&target_path).await?;

        Ok(())
    }

    /// Update desktop integration for AppImage
    async fn update_desktop_integration(&self, appimage_path: &Path) -> anyhow::Result<()> {
        let desktop_file = dirs::home_dir()
            .unwrap()
            .join(".local/share/applications/easyssh.desktop");

        let desktop_content = format!(
            r#"[Desktop Entry]
Name=EasySSH
Comment=SSH Client
Exec={}
Icon=easyssh
Type=Application
Categories=Network;RemoteAccess;
Terminal=false
"#,
            appimage_path.display()
        );

        tokio::fs::create_dir_all(desktop_file.parent().unwrap()).await?;
        tokio::fs::write(&desktop_file, desktop_content).await?;

        // Update desktop database
        Command::new("update-desktop-database")
            .arg(dirs::home_dir().unwrap().join(".local/share/applications"))
            .output()?;

        Ok(())
    }

    /// Install Flatpak
    async fn install_flatpak(&self, flatpak_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("flatpak")
            .args(&["install", "-y", "--bundle", flatpak_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Flatpak installation failed"));
        }

        Ok(())
    }

    /// Install Snap
    async fn install_snap(&self, snap_path: &Path) -> anyhow::Result<()> {
        let output = Command::new("sudo")
            .args(&["snap", "install", "--dangerous", snap_path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Snap installation failed"));
        }

        Ok(())
    }

    /// Install portable binary
    async fn install_portable(&self, binary_path: &Path) -> anyhow::Result<()> {
        let target_path = self.install_dir.join("easyssh");

        // Backup current binary
        if target_path.exists() {
            let backup = target_path.with_extension(
                format!("backup.{}", chrono::Local::now().timestamp())
            );
            tokio::fs::copy(&target_path, &backup).await?;
        }

        // Copy new binary
        tokio::fs::copy(binary_path, &target_path).await?;

        // Make executable
        tokio::fs::set_permissions(
            &target_path,
            std::fs::Permissions::from_mode(0o755),
        ).await?;

        Ok(())
    }

    /// Extract tarball
    async fn extract_tarball(&self, tar_path: &Path) -> anyhow::Result<PathBuf> {
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

        // Find the binary in extracted directory
        for entry in walkdir::WalkDir::new(&extract_dir).max_depth(2) {
            let entry = entry?;
            if entry.file_name() == "easyssh" {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(anyhow::anyhow!("Binary not found in tarball"))
    }

    /// Get package info from DEB
    pub async fn get_deb_info(deb_path: &Path) -> anyhow::Result<HashMap<String, String>> {
        let output = Command::new("dpkg-deb")
            .args(&["-I", deb_path.to_str().unwrap()])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut info = HashMap::new();

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(':') {
                info.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        Ok(info)
    }

    /// Get package info from RPM
    pub async fn get_rpm_info(rpm_path: &Path) -> anyhow::Result<HashMap<String, String>> {
        let output = Command::new("rpm")
            .args(&["-qpi", rpm_path.to_str().unwrap()])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut info = HashMap::new();

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(':') {
                info.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        Ok(info)
    }

    /// Check if package is already installed
    pub async fn is_package_installed(&self, package_name: &str) -> anyhow::Result<bool> {
        let result = match self.package_manager {
            PackageManager::Apt => {
                Command::new("dpkg")
                    .args(&["-l", package_name])
                    .output()?
                    .status
                    .success()
            }
            PackageManager::Dnf | PackageManager::Yum => {
                Command::new("rpm")
                    .args(&["-q", package_name])
                    .output()?
                    .status
                    .success()
            }
            PackageManager::Pacman => {
                Command::new("pacman")
                    .args(&["-Q", package_name])
                    .output()?
                    .status
                    .success()
            }
            _ => false,
        };

        Ok(result)
    }
}

use std::collections::HashMap;

#[async_trait]
impl PlatformUpdater for LinuxUpdater {
    fn get_package_extension(&self) -> &'static str {
        match self.package_manager {
            PackageManager::Apt => "deb",
            PackageManager::Dnf | PackageManager::Yum | PackageManager::Zypper => "rpm",
            PackageManager::Pacman => "pkg.tar.zst",
            PackageManager::AppImage => "AppImage",
            PackageManager::Flatpak => "flatpak",
            PackageManager::Snap => "snap",
            PackageManager::Portable => "tar.gz",
        }
    }

    async fn install_update(&self, package_path: &Path) -> anyhow::Result<()> {
        match self.package_manager {
            PackageManager::Apt => self.install_apt(package_path).await,
            PackageManager::Dnf => self.install_dnf(package_path).await,
            PackageManager::Yum => self.install_dnf(package_path).await,
            PackageManager::Pacman => self.install_pacman(package_path).await,
            PackageManager::Zypper => self.install_zypper(package_path).await,
            PackageManager::AppImage => self.install_appimage(package_path).await,
            PackageManager::Flatpak => self.install_flatpak(package_path).await,
            PackageManager::Snap => self.install_snap(package_path).await,
            PackageManager::Portable => {
                let binary = self.extract_tarball(package_path).await?;
                self.install_portable(&binary).await
            }
        }
    }

    async fn restart_application(&self) -> anyhow::Result<()> {
        let current_exe = std::env::current_exe()?;

        // Use execvp equivalent - replace current process
        Command::new(&current_exe)
            .spawn()?;

        std::process::exit(0);
    }

    async fn needs_elevation(&self, _package_path: &Path) -> anyhow::Result<bool> {
        // Check if we need sudo
        match self.package_manager {
            PackageManager::Apt |
            PackageManager::Dnf |
            PackageManager::Yum |
            PackageManager::Pacman |
            PackageManager::Zypper |
            PackageManager::Snap => Ok(true),
            _ => {
                // Check write permission to install dir
                let test_file = self.install_dir.join(".write_test");
                match tokio::fs::write(&test_file, b"").await {
                    Ok(_) => {
                        let _ = tokio::fs::remove_file(&test_file).await;
                        Ok(false)
                    }
                    Err(_) => Ok(true),
                }
            }
        }
    }

    fn get_current_executable(&self) -> anyhow::Result<std::path::PathBuf> {
        std::env::current_exe()
    }

    async fn verify_package(&self, package_path: &Path) -> anyhow::Result<bool> {
        match self.package_manager {
            PackageManager::Apt => {
                // Verify DEB signature
                let output = Command::new("dpkg-sig")
                    .args(&["--verify", package_path.to_str().unwrap()])
                    .output()?;
                Ok(output.status.success())
            }
            PackageManager::Dnf | PackageManager::Yum => {
                // Verify RPM signature
                let output = Command::new("rpm")
                    .args(&["-K", package_path.to_str().unwrap()])
                    .output()?;
                Ok(output.status.success())
            }
            _ => Ok(true), // Other formats use our own signature verification
        }
    }
}

/// Get distribution name
pub async fn get_distribution() -> String {
    // Try /etc/os-release
    if let Ok(content) = tokio::fs::read_to_string("/etc/os-release").await {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("ID=") {
                return value.trim_matches('"').to_string();
            }
        }
    }

    // Fallback to lsb_release
    if let Ok(output) = Command::new("lsb_release")
        .args(&["-is"])
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
    }

    "unknown".to_string()
}

/// Get distribution version
pub async fn get_distribution_version() -> String {
    if let Ok(content) = tokio::fs::read_to_string("/etc/os-release").await {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("VERSION_ID=") {
                return value.trim_matches('"').to_string();
            }
        }
    }

    if let Ok(output) = Command::new("lsb_release")
        .args(&["-rs"])
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }

    "unknown".to_string()
}
