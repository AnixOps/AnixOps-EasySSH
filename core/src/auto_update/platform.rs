//! Platform-specific update implementations

use async_trait::async_trait;
use std::fmt::Debug;
use std::path::Path;
use std::process::Command;

pub mod windows;
pub mod macos;
pub mod linux;

#[async_trait]
pub trait PlatformUpdater: Send + Sync + Debug {
    /// Get the package extension for this platform
    fn get_package_extension(&self) -> &'static str;

    /// Install the update package
    async fn install_update(&self, package_path: &Path) -> anyhow::Result<()>;

    /// Restart the application after update
    async fn restart_application(&self) -> anyhow::Result<()>;

    /// Check if elevated privileges are needed
    async fn needs_elevation(&self, package_path: &Path) -> anyhow::Result<bool>;

    /// Get the currently running executable path
    fn get_current_executable(&self) -> anyhow::Result<std::path::PathBuf>;

    /// Verify package signature (platform-specific)
    async fn verify_package(&self, package_path: &Path) -> anyhow::Result<bool>;
}

/// Create platform-specific updater
pub async fn create_platform_updater() -> anyhow::Result<Box<dyn PlatformUpdater>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsUpdater::new().await?))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSUpdater::new().await?))
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxUpdater::new().await?))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform for auto-update")
    }
}

/// Helper function to run command with elevated privileges on Windows
#[cfg(target_os = "windows")]
pub async fn run_elevated_windows(command: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    const SW_HIDE: u32 = 0;

    let mut cmd = Command::new("runas");
    cmd.arg("/user:Administrator")
        .arg(command)
        .args(args)
        .creation_flags(0x08000000); // CREATE_NO_WINDOW

    Ok(cmd.output()?)
}

/// Helper function to run command with sudo on Unix
#[cfg(not(target_os = "windows"))]
pub async fn run_with_sudo(command: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let output = Command::new("sudo")
        .arg("-n") // Non-interactive
        .arg(command)
        .args(args)
        .output()?;

    Ok(output)
}

/// Helper to copy file with progress callback
pub async fn copy_file_with_progress(
    src: &Path,
    dst: &Path,
    _callback: impl Fn(u64, u64),
) -> anyhow::Result<()> {
    tokio::fs::copy(src, dst).await?;
    Ok(())
}

/// Helper to safely replace executable
pub async fn replace_executable(
    current: &Path,
    new: &Path,
    backup: Option<&Path>,
) -> anyhow::Result<()> {
    // Create backup if requested
    if let Some(backup_path) = backup {
        tokio::fs::copy(current, backup_path).await?;
    }

    // On Windows, we can't replace a running executable directly
    // We need to use a batch script or MoveFileEx with MOVEFILE_DELAY_UNTIL_REBOOT
    #[cfg(target_os = "windows")]
    {
        windows::replace_executable_windows(current, new).await
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, we can atomically replace
        tokio::fs::rename(new, current).await?;
        Ok(())
    }
}
